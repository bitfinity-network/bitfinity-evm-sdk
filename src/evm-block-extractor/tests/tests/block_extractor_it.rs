use std::collections::BTreeMap;
use std::future::Future;
use std::ops::Range;
use std::pin::Pin;
use std::sync::Arc;

use did::evm_state::EvmGlobalState;
use did::{
    keccak, BlockConfirmationData, BlockConfirmationResult, BlockNumber, BlockchainBlockInfo, H160,
};
use ethereum_json_rpc_client::reqwest::ReqwestClient;
use ethereum_json_rpc_client::{CertifiedResult, Client, EthJsonRpcClient};
use evm_block_extractor::database::AccountBalance;
use evm_block_extractor::server;
use evm_block_extractor::task::block_extractor::{BlockExtractCollectOutcome, BlockExtractor};
use jsonrpc_core::{Call, Failure, Output, Params, Request, Response};
use serde::de::DeserializeOwned;

use crate::test_with_clients;

#[tokio::test]
async fn test_extractor_collect_blocks() {
    test_with_clients(|db_client| async move {
        db_client.init(None, true).await.unwrap();

        let start_block = 0;
        let end_block = 100;
        let blocks =
            generate_correct_block_sequence(start_block..end_block + 1, Default::default(), 10);
        let mock_client = MockClient::with_blocks(EvmGlobalState::Enabled, blocks.clone());
        let mock_evm_client = Arc::new(EthJsonRpcClient::new(mock_client));
        let request_time_out_secs = 10;
        let rpc_batch_size = 10;
        let mut extractor = BlockExtractor::new(
            mock_evm_client.clone(),
            request_time_out_secs,
            rpc_batch_size,
            db_client.clone(),
        );

        println!("Getting blocks from {:?} to {}", start_block, end_block);

        let result = extractor.collect_all(start_block, end_block).await.unwrap();

        match result {
            BlockExtractCollectOutcome::BlocksExtracted {
                from_block,
                to_block,
            } => {
                assert_eq!(from_block, start_block);
                assert_eq!(to_block, end_block);
            }
            _ => panic!("Expected BlocksExtracted"),
        }

        // Check genesis accounts
        {
            let evmc_genesis_balances = mock_evm_client.get_genesis_balances().await.unwrap();
            let db_genesis_balances = db_client.get_genesis_balances().await.unwrap().unwrap();

            assert!(!evmc_genesis_balances.is_empty());

            let evmc_genesis_balances = evmc_genesis_balances
                .into_iter()
                .map(|(address, balance)| AccountBalance { address, balance })
                .collect::<Vec<_>>();

            assert_eq!(evmc_genesis_balances, db_genesis_balances);
        }

        // Check chain id
        {
            let evmc_chain_id = mock_evm_client.get_chain_id().await.unwrap();
            let db_chain_id = db_client.get_chain_id().await.unwrap().unwrap();

            assert_eq!(evmc_chain_id, db_chain_id);
        }

        // Check last certified block
        {
            let certified_data = db_client.get_last_certified_block_data().await.unwrap();
            assert!(!certified_data.certificate.is_empty());
            assert!(!certified_data.witness.is_empty());

            // Check that it is more or less last block
            assert!(end_block - 10 <= certified_data.data.number.0.to::<u64>());
            assert!(end_block + 10 >= certified_data.data.number.0.to::<u64>());
        }

        let blocks_map = BTreeMap::from_iter(blocks.into_iter().map(|b| (b.number.as_u64(), b)));
        for block_num in start_block..=end_block {
            let block = db_client.get_block_by_number(block_num).await.unwrap();

            let full_block = db_client.get_full_block_by_number(block_num).await.unwrap();

            let source_block = &blocks_map[&block_num];
            // Check blocks
            {
                assert_eq!(source_block.hash, full_block.hash);
                assert_eq!(block_num, full_block.number.0.to::<u64>());
                assert_eq!(block_num, block.number.0.to::<u64>());
                assert_eq!(block.hash, full_block.hash);
            }

            // Check transactions
            {
                println!(
                    "Found transactions for block {}: {}",
                    block_num,
                    block.transactions.len()
                );
                assert_eq!(block.transactions.len(), full_block.transactions.len());
                assert_eq!(
                    source_block.transactions.len(),
                    full_block.transactions.len()
                );

                let simple_source_block: did::Block<did::H256> = source_block.clone().into();
                for tx in &full_block.transactions {
                    assert!(block.transactions.contains(&tx.hash));
                    assert!(simple_source_block.transactions.contains(&tx.hash));
                    assert_eq!(tx.block_number, tx.block_number);
                    assert_eq!(tx.block_hash, tx.block_hash);
                }
            }
        }
    })
    .await;
}

const CHAIN_ID: u64 = 42;

#[derive(Clone)]
pub struct MockClient {
    evm_global_state: EvmGlobalState,
    blocks: BTreeMap<u64, did::Block<did::Transaction>>,
}

impl MockClient {
    pub fn new(evm_global_state: EvmGlobalState) -> Self {
        Self {
            evm_global_state,
            blocks: BTreeMap::new(),
        }
    }

    /// Create a new mock client with the given blocks in storage.
    pub fn with_blocks<I>(evm_global_state: EvmGlobalState, blocks: I) -> Self
    where
        I: IntoIterator<Item = did::Block<did::Transaction>>,
    {
        Self {
            evm_global_state,
            blocks: blocks.into_iter().map(|b| (b.number.as_u64(), b)).collect(),
        }
    }

    fn process_single_call(&self, call: Call) -> Output {
        match call {
            Call::MethodCall(method_call) => match method_call.method.as_str() {
                "ic_getEvmGlobalState" => Output::Success(jsonrpc_core::Success {
                    jsonrpc: None,
                    result: serde_json::to_value(&self.evm_global_state).unwrap(),
                    id: method_call.id,
                }),
                "eth_getBlockByNumber" => {
                    let number: BlockNumber = Self::get_from_vec(&method_call.params, 0);
                    let block = match number {
                        BlockNumber::Latest | BlockNumber::Finalized | BlockNumber::Safe => {
                            self.blocks.last_key_value().map(|(_, v)| v.clone())
                        }
                        BlockNumber::Earliest => {
                            self.blocks.last_key_value().map(|(_, v)| v.clone())
                        }
                        BlockNumber::Pending => unimplemented!(),
                        BlockNumber::Number(n) => self.blocks.get(&n.as_u64()).cloned(),
                    };
                    match block {
                        Some(block) => Output::Success(jsonrpc_core::Success {
                            jsonrpc: None,
                            result: serde_json::to_value(block).unwrap(),
                            id: method_call.id,
                        }),
                        None => Output::Failure(Failure {
                            jsonrpc: None,
                            error: jsonrpc_core::Error::invalid_params("block not found"),
                            id: method_call.id,
                        }),
                    }
                }
                "ic_getLastCertifiedBlock" => {
                    let data = self
                        .blocks
                        .last_key_value()
                        .map(|(_, v)| v)
                        .cloned()
                        .unwrap_or_else(|| did::Block::default().into_full_block(vec![]).unwrap());
                    Output::Success(jsonrpc_core::Success {
                        jsonrpc: None,
                        result: serde_json::to_value(&CertifiedResult {
                            data,
                            witness: vec![4, 5, 6u8],
                            certificate: vec![7, 8, 9],
                        })
                        .unwrap(),
                        id: method_call.id,
                    })
                }
                "eth_chainId" => Output::Success(jsonrpc_core::Success {
                    jsonrpc: None,
                    result: serde_json::to_value(CHAIN_ID.to_string()).unwrap(),
                    id: method_call.id,
                }),
                "ic_sendConfirmBlock" => Output::Success(jsonrpc_core::Success {
                    jsonrpc: None,
                    result: serde_json::to_value(BlockConfirmationResult::Confirmed).unwrap(),
                    id: method_call.id,
                }),
                "ic_getGenesisBalances" => {
                    let balances: Vec<_> =
                        (1..=32).map(|i| (i32_to_h160(i), i32_to_h256(i))).collect();
                    Output::Success(jsonrpc_core::Success {
                        jsonrpc: None,
                        result: serde_json::to_value(balances).unwrap(),
                        id: method_call.id,
                    })
                }
                "ic_getBlockchainBlockInfo" => Output::Success(jsonrpc_core::Success {
                    jsonrpc: None,
                    result: serde_json::to_value(BlockchainBlockInfo {
                        earliest_block_number: self
                            .blocks
                            .first_key_value()
                            .map(|(k, _)| *k)
                            .unwrap_or_default(),
                        latest_block_number: self
                            .blocks
                            .last_key_value()
                            .map(|(k, _)| *k)
                            .unwrap_or_default(),
                        safe_block_number: self
                            .blocks
                            .last_key_value()
                            .map(|(k, _)| *k)
                            .unwrap_or_default(),
                        finalized_block_number: self
                            .blocks
                            .last_key_value()
                            .map(|(k, _)| *k)
                            .unwrap_or_default(),
                        pending_block_number: self
                            .blocks
                            .last_key_value()
                            .map(|(k, _)| *k + 1)
                            .unwrap_or_default(),
                    })
                    .unwrap(),
                    id: method_call.id,
                }),
                _ => unimplemented!(),
            },
            _ => unimplemented!(),
        }
    }

    fn get_from_vec<T: DeserializeOwned>(params: &Params, index: usize) -> T {
        let Params::Array(params) = params else {
            panic!("missing params");
        };

        match params.get(index) {
            Some(value) => serde_json::from_value(value.clone()).unwrap(),
            None => panic!("index {} exceeds length of params {}", index, params.len()),
        }
    }
}

fn i32_to_h160(i: i32) -> did::H160 {
    let mut buf = [0; 20];
    buf[..4].copy_from_slice(&i.to_be_bytes());
    H160::from_slice(&buf)
}

fn i32_to_h256(i: i32) -> did::H256 {
    keccak::keccak_hash(&i.to_be_bytes())
}

impl Client for MockClient {
    fn send_rpc_request(
        &self,
        request: Request,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<Response>> + Send>> {
        let response = match request {
            Request::Single(call) => Response::Single(self.process_single_call(call)),
            Request::Batch(calls) => {
                let processed = calls
                    .into_iter()
                    .map(|c| self.process_single_call(c))
                    .collect();

                Response::Batch(processed)
            }
        };
        Box::pin(async { Ok(response) })
    }
}

#[tokio::test]
async fn test_mock_client_returns_evm_state() {
    let client = MockClient::new(EvmGlobalState::Enabled);

    let response = client
        .send_rpc_request(Request::Single(Call::MethodCall(
            jsonrpc_core::MethodCall {
                jsonrpc: None,
                method: "ic_getEvmGlobalState".to_string(),
                params: jsonrpc_core::Params::None,
                id: jsonrpc_core::Id::Num(1),
            },
        )))
        .await
        .unwrap();

    assert_eq!(
        response,
        Response::Single(Output::Success(jsonrpc_core::Success {
            jsonrpc: None,
            result: serde_json::to_value(EvmGlobalState::Enabled).unwrap(),
            id: jsonrpc_core::Id::Num(1),
        }))
    );
}

#[tokio::test]
async fn test_extractor_does_not_collect_blocks_if_evm_is_disabled() {
    test_with_clients(|db_client| async move {
        db_client.init(None, true).await.unwrap();

        let client = MockClient::new(EvmGlobalState::Disabled);

        let evm_client = Arc::new(EthJsonRpcClient::new(client));

        let mut extractor = BlockExtractor::new(evm_client.clone(), 10, 10, db_client.clone());

        let result = extractor.collect_all(100, 1000).await.unwrap();

        match result {
            BlockExtractCollectOutcome::BlocksNotExtracted => {}
            _ => panic!("Expected BlocksNotExtracted"),
        }
    })
    .await;
}

#[tokio::test]
async fn test_extractor_does_not_collect_blocks_if_evm_is_staging() {
    test_with_clients(|db_client| async move {
        db_client.init(None, true).await.unwrap();

        let client = MockClient::new(EvmGlobalState::Staging {
            max_block_number: None,
        });

        let evm_client = Arc::new(EthJsonRpcClient::new(client));

        let mut extractor = BlockExtractor::new(evm_client.clone(), 10, 10, db_client.clone());

        let result = extractor.collect_all(100, 1000).await.unwrap();

        match result {
            BlockExtractCollectOutcome::BlocksNotExtracted => {}
            _ => panic!("Expected BlocksNotExtracted"),
        }
    })
    .await;
}

#[tokio::test]
async fn test_extractor_validate_and_recover_blockchain() {
    test_with_clients(|db_client| async move {
        db_client.init(None, true).await.unwrap();

        let start_block = 10;
        let end_block = 20;
        let blocks =
            generate_correct_block_sequence(start_block..end_block + 1, Default::default(), 10);
        let init_blocks_last_hash = blocks.last().cloned().unwrap().hash;
        let mock_client = MockClient::with_blocks(EvmGlobalState::Enabled, blocks);
        let mock_evm_client = Arc::new(EthJsonRpcClient::new(mock_client));

        let request_time_out_secs = 10;
        let rpc_batch_size = 10;
        let mut extractor = BlockExtractor::new(
            mock_evm_client,
            request_time_out_secs,
            rpc_batch_size,
            db_client.clone(),
        );

        log::info!("Getting blocks from {:?} to {}", start_block, end_block);

        let result = extractor.collect_all(start_block, end_block).await.unwrap();

        match result {
            BlockExtractCollectOutcome::BlocksExtracted {
                from_block,
                to_block,
            } => {
                assert_eq!(from_block, start_block);
                assert_eq!(to_block, end_block);
            }
            _ => panic!("Expected BlocksExtracted"),
        }

        // Add several broken blocks to DB
        let block_before_broken = end_block;
        let first_broken_block = end_block + 1;
        let last_broken_block = end_block + 5;
        let broken_blocks = generate_correct_block_sequence(
            first_broken_block..last_broken_block,
            init_blocks_last_hash.clone(),
            10,
        );
        let txs: Vec<_> = broken_blocks
            .iter()
            .flat_map(|b| &b.transactions)
            .cloned()
            .collect();
        let broken_blocks: Vec<_> = broken_blocks.into_iter().map(Into::into).collect();
        db_client
            .insert_block_data(&broken_blocks, &txs)
            .await
            .unwrap();

        let start_block = last_broken_block + 1;
        let end_block = last_broken_block + 10;
        let blocks = generate_correct_block_sequence(
            start_block..end_block + 1,
            keccak::keccak_hash(&[1, 2]),
            10,
        );
        let mock_client = MockClient::with_blocks(EvmGlobalState::Enabled, blocks);
        let mock_evm_client = Arc::new(EthJsonRpcClient::new(mock_client));
        let mut extractor = BlockExtractor::new(
            mock_evm_client,
            request_time_out_secs,
            rpc_batch_size,
            db_client.clone(),
        );
        let collect_result = extractor.collect_all(start_block, end_block).await;
        assert!(collect_result.is_err());

        // check the blockchain state recovered: invalid blocks discarded
        for block in &broken_blocks {
            let block_result = db_client.get_block_by_number(block.number.as_u64()).await;
            assert!(block_result.is_err());

            let block_result = db_client
                .get_discarded_block_by_hash(block.hash.clone())
                .await
                .unwrap();

            for tx_hash in &block.transactions {
                let tx_result = db_client.get_transaction(tx_hash.clone()).await;
                assert!(tx_result.is_err());

                let discarded_tx = block_result
                    .block
                    .transactions
                    .iter()
                    .find(|tx| &tx.hash == tx_hash);
                assert!(discarded_tx.is_some());
            }
        }

        let last_block_after_recovery = db_client.get_latest_block_number().await.unwrap();
        assert_eq!(last_block_after_recovery, Some(block_before_broken));

        let start_block = block_before_broken + 1;
        let end_block = start_block + 10;
        let blocks =
            generate_correct_block_sequence(start_block..end_block + 1, init_blocks_last_hash, 10);
        let mock_client = MockClient::with_blocks(EvmGlobalState::Enabled, blocks);
        let mock_evm_client = Arc::new(EthJsonRpcClient::new(mock_client));
        let mut extractor = BlockExtractor::new(
            mock_evm_client,
            request_time_out_secs,
            rpc_batch_size,
            db_client.clone(),
        );
        extractor.collect_all(start_block, end_block).await.unwrap();
        let last_block = db_client.get_latest_block_number().await.unwrap();
        assert_eq!(last_block, Some(end_block));
    })
    .await;
}

#[tokio::test]
async fn test_extractor_skips_incorrect_sequence_of_new_blocks() {
    test_with_clients(|db_client| async move {
        db_client.init(None, true).await.unwrap();

        let start_block = 10;
        let end_block = 20;
        let blocks =
            generate_correct_block_sequence(start_block..end_block + 1, Default::default(), 10);
        let init_blocks_last_hash = blocks.last().cloned().unwrap().hash;
        let mock_client = MockClient::with_blocks(EvmGlobalState::Enabled, blocks);
        let mock_evm_client = Arc::new(EthJsonRpcClient::new(mock_client));

        let request_time_out_secs = 10;
        let rpc_batch_size = 10;
        let mut extractor = BlockExtractor::new(
            mock_evm_client,
            request_time_out_secs,
            rpc_batch_size,
            db_client.clone(),
        );

        log::info!("Getting blocks from {:?} to {}", start_block, end_block);

        let result = extractor.collect_all(start_block, end_block).await.unwrap();

        match result {
            BlockExtractCollectOutcome::BlocksExtracted {
                from_block,
                to_block,
            } => {
                assert_eq!(from_block, start_block);
                assert_eq!(to_block, end_block);
            }
            _ => panic!("Expected BlocksExtracted"),
        }

        // Try to add broken blocks sequence
        const BROKEN_BLOCKS_NUMBER: u64 = 4;
        let first_broken_block_number = end_block + 1;
        let last_broken_block_number = first_broken_block_number + BROKEN_BLOCKS_NUMBER;

        let mut broken_blocks = generate_correct_block_sequence(
            first_broken_block_number..last_broken_block_number + 1,
            init_blocks_last_hash.clone(),
            10,
        );

        broken_blocks[2].parent_hash = keccak::keccak_hash(&[1, 2, 3]);

        let mock_client = MockClient::with_blocks(EvmGlobalState::Enabled, broken_blocks.clone());
        let mock_evm_client = Arc::new(EthJsonRpcClient::new(mock_client));
        let mut extractor = BlockExtractor::new(
            mock_evm_client,
            request_time_out_secs,
            rpc_batch_size,
            db_client.clone(),
        );

        let collect_result = extractor
            .collect_all(first_broken_block_number, last_broken_block_number)
            .await;
        // broken blocks sequence should be ignored.
        assert!(collect_result.is_err());

        // check the blockchain state is not changed: invalid blocks sequence not stored
        for block in &broken_blocks {
            let block_result = db_client.get_block_by_number(block.number.as_u64()).await;
            assert!(block_result.is_err());

            let block_result = db_client
                .get_discarded_block_by_hash(block.hash.clone())
                .await;
            assert!(block_result.is_err());

            for tx in &block.transactions {
                let tx_result = db_client.get_transaction(tx.hash.clone()).await;
                assert!(tx_result.is_err());
            }
        }

        let last_block_after_incorrect_blocks_ignored =
            db_client.get_latest_block_number().await.unwrap();
        assert_eq!(last_block_after_incorrect_blocks_ignored, Some(end_block));

        // Try to collect correct blocks.
        let start_block = 21;
        let end_block = 30;
        let blocks =
            generate_correct_block_sequence(start_block..end_block + 1, init_blocks_last_hash, 10);
        let mock_client = MockClient::with_blocks(EvmGlobalState::Enabled, blocks);
        let mock_evm_client = Arc::new(EthJsonRpcClient::new(mock_client));
        let mut extractor = BlockExtractor::new(
            mock_evm_client,
            request_time_out_secs,
            rpc_batch_size,
            db_client.clone(),
        );
        extractor.collect_all(start_block, end_block).await.unwrap();
        let last_block = db_client.get_latest_block_number().await.unwrap();
        assert_eq!(last_block, Some(end_block));
    })
    .await;
}

#[tokio::test]
async fn test_server_returns_blocks_according_to_tags() {
    test_with_clients(|db_client| async move {
        db_client.init(None, true).await.unwrap();

        // fill db with blocks
        let block_numbers = 1..100;
        let blocks = generate_correct_block_sequence(block_numbers.clone(), Default::default(), 10);
        let txs: Vec<_> = blocks
            .iter()
            .flat_map(|b| &b.transactions)
            .cloned()
            .collect();
        let blocks: Vec<_> = blocks.into_iter().map(Into::into).collect();
        db_client.insert_block_data(&blocks, &txs).await.unwrap();
        let block_info = BlockchainBlockInfo {
            earliest_block_number: 0,
            latest_block_number: 1000,
            safe_block_number: 90,
            finalized_block_number: 95,
            pending_block_number: 123,
        };
        db_client.set_block_info(block_info.clone()).await.unwrap();

        let addr = "127.0.0.1:49764";
        let client = Arc::new(EthJsonRpcClient::new(MockClient::new(
            EvmGlobalState::Enabled,
        )));
        let _server = server::server_start(addr, db_client.clone(), client)
            .await
            .unwrap();
        let extractor_client = EthJsonRpcClient::new(ReqwestClient::new(format!("http://{addr}")));

        let block = extractor_client
            .get_block_by_number(BlockNumber::Latest)
            .await
            .unwrap();
        assert_eq!(block.number.as_u64(), block_numbers.end - 1);

        let block = extractor_client
            .get_block_by_number(BlockNumber::Earliest)
            .await
            .unwrap();
        assert_eq!(block.number.as_u64(), block_numbers.start);

        let block = extractor_client
            .get_block_by_number(BlockNumber::Safe)
            .await
            .unwrap();
        assert_eq!(block.number.as_u64(), block_info.safe_block_number);

        let block = extractor_client
            .get_block_by_number(BlockNumber::Finalized)
            .await
            .unwrap();
        assert_eq!(block.number.as_u64(), block_info.finalized_block_number);

        // when safe and finalized blocks are not extracted, server should return latest block in storage.
        let block_info = BlockchainBlockInfo {
            earliest_block_number: 0,
            latest_block_number: 1000,
            safe_block_number: 1000,
            finalized_block_number: 1000,
            pending_block_number: 123,
        };
        db_client.set_block_info(block_info).await.unwrap();

        let block = extractor_client
            .get_block_by_number(BlockNumber::Safe)
            .await
            .unwrap();
        assert_eq!(block.number.as_u64(), block_numbers.end - 1);

        let block = extractor_client
            .get_block_by_number(BlockNumber::Finalized)
            .await
            .unwrap();
        assert_eq!(block.number.as_u64(), block_numbers.end - 1);
    })
    .await
}

#[tokio::test]
async fn test_extractor_forwards_confirm_block_requests() {
    test_with_clients(|db_client| async move {
        db_client.init(None, true).await.unwrap();

        let block_info = BlockchainBlockInfo {
            earliest_block_number: 0,
            latest_block_number: 1000,
            safe_block_number: 90,
            finalized_block_number: 95,
            pending_block_number: 123,
        };
        db_client.set_block_info(block_info.clone()).await.unwrap();

        let addr = "127.0.0.1:49763";
        let client = Arc::new(EthJsonRpcClient::new(MockClient::new(
            EvmGlobalState::Enabled,
        )));
        let _server = server::server_start(addr, db_client.clone(), client)
            .await
            .unwrap();
        let extractor_client = EthJsonRpcClient::new(ReqwestClient::new(format!("http://{addr}")));

        // Should not be forwarded
        let confirm_result = extractor_client
            .send_confirm_block(BlockConfirmationData {
                block_number: 90,
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(confirm_result, BlockConfirmationResult::AlreadyConfirmed);

        // Should be forwarded
        let confirm_result = extractor_client
            .send_confirm_block(BlockConfirmationData {
                block_number: 100,
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(confirm_result, BlockConfirmationResult::Confirmed);
    })
    .await
}

fn generate_correct_block_sequence(
    ids: Range<u64>,
    parent_hash: did::H256,
    txs_per_block: usize,
) -> Vec<did::Block<did::Transaction>> {
    if ids.end <= ids.start {
        return vec![];
    }

    let mut blocks = ids
        .map(|id| did::Block {
            number: id.into(),
            hash: i32_to_h256(id as _),
            ..Default::default()
        })
        .collect::<Vec<_>>();

    blocks[0].parent_hash = parent_hash;

    for i in 1..blocks.len() {
        blocks[i].parent_hash = blocks[i - 1].hash.clone();
    }

    let mut blocks_with_txs = vec![];
    for block in blocks {
        let mut txs = vec![];
        for j in 0..txs_per_block {
            let tx_num = (block.number.as_u64() << 4) + j as u64;
            let tx_hash = keccak::keccak_hash(&tx_num.to_be_bytes());
            let block_number = block.number.0.to::<u64>();
            let dummy_tx = did::Transaction {
                hash: tx_hash,
                block_number: Some(did::U64::from(block_number)),
                ..Default::default()
            };

            txs.push(dummy_tx);
        }
        blocks_with_txs.push(block.clone().into_full_block(txs).unwrap());
    }

    blocks_with_txs
}
