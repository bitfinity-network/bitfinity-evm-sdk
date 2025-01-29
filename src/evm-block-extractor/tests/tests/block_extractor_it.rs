use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use did::evm_state::EvmGlobalState;
use ethereum_json_rpc_client::reqwest::ReqwestClient;
use ethereum_json_rpc_client::{Client, EthJsonRpcClient};
use evm_block_extractor::database::AccountBalance;
use evm_block_extractor::task::block_extractor::{BlockExtractCollectOutcome, BlockExtractor};
use jsonrpc_core::{Call, Output, Request, Response};

use crate::test_with_clients;

#[tokio::test]
async fn test_extractor_collect_blocks() {
    test_with_clients(|db_client| async move {
        db_client.init(None, true).await.unwrap();

        let rpc_url =
            "https://block-extractor-testnet-1052151659755.europe-west9.run.app".to_string();
        let evm_client = Arc::new(EthJsonRpcClient::new(ReqwestClient::new(rpc_url)));

        let request_time_out_secs = 10;
        let rpc_batch_size = 10;
        let mut extractor = BlockExtractor::new(
            evm_client.clone(),
            request_time_out_secs,
            rpc_batch_size,
            db_client.clone(),
        );

        let end_block = evm_client.get_block_number().await.unwrap();
        let start_block = end_block - 10;

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
            let evmc_genesis_balances = evm_client.get_genesis_balances().await.unwrap();
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
            let evmc_chain_id = evm_client.get_chain_id().await.unwrap();
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

        for block_num in start_block..=end_block {
            let block = db_client.get_block_by_number(block_num).await.unwrap();

            let full_block = db_client.get_full_block_by_number(block_num).await.unwrap();

            // Check blocks
            {
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

                for tx in &full_block.transactions {
                    assert!(block.transactions.contains(&tx.hash));
                    assert_eq!(tx.block_number, tx.block_number);
                    assert_eq!(tx.block_hash, tx.block_hash);
                }
            }
        }
    })
    .await;
}

#[derive(Clone)]
pub struct MockClient {
    evm_global_state: EvmGlobalState,
}

impl MockClient {
    /// Create a new mock client
    pub fn new(evm_global_state: EvmGlobalState) -> Self {
        Self { evm_global_state }
    }
}

impl Client for MockClient {
    fn send_rpc_request(
        &self,
        request: Request,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<Response>> + Send>> {
        let response = match request {
            Request::Single(call) => match call {
                Call::MethodCall(method_call) => match method_call.method.as_str() {
                    "ic_getEvmGlobalState" => {
                        Response::Single(Output::Success(jsonrpc_core::Success {
                            jsonrpc: None,
                            result: serde_json::to_value(&self.evm_global_state).unwrap(),
                            id: jsonrpc_core::Id::Num(1),
                        }))
                    }
                    _ => unimplemented!(),
                },
                _ => unimplemented!(),
            },
            _ => unimplemented!(),
        };
        Box::pin(async { Ok(response) })
    }
}

#[tokio::test]
async fn test_mock_client_returns_evm_state() {
    let client = MockClient {
        evm_global_state: EvmGlobalState::Enabled,
    };

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

        let client = MockClient {
            evm_global_state: EvmGlobalState::Disabled,
        };

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

        let client = MockClient {
            evm_global_state: EvmGlobalState::Staging {
                max_block_number: None,
            },
        };

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
