use std::future::Future;
use std::sync::Arc;

use did::{Block, H160, U256, U64};
use ethereum_json_rpc_client::reqwest::ReqwestClient;
use ethereum_json_rpc_client::{Client, EthJsonRpcClient};
use ethers_core::types::{BlockNumber, Transaction, H256};
use evm_block_extractor::database::{AccountBalance, DatabaseClient};
use evm_block_extractor::rpc::{EthImpl, EthServer, ICServer};
use jsonrpc_core::{Call, Id, MethodCall, Output, Params, Request, Response, Version};
use jsonrpsee::server::{Server, ServerHandle};
use jsonrpsee::RpcModule;
use rand::random;
use serde_json::json;

use crate::test_with_clients;

const BLOCK_COUNT: u64 = 10;

async fn with_filled_db<Func: Fn(Arc<dyn DatabaseClient>) -> Fut, Fut: Future<Output = ()>>(
    func: Func,
) {
    test_with_clients(|db_client| async {
        db_client.init(None, false).await.unwrap();

        for i in 0..BLOCK_COUNT {
            let tx_hash = H256::random();
            let dummy_transaction: did::Transaction = Transaction {
                hash: tx_hash,
                block_number: Some(i.into()),
                ..Default::default()
            }
            .into();
            let dummy_block: did::Block<did::H256> = ethers_core::types::Block::<H256> {
                number: Some(ethers_core::types::U64::from(i)),
                hash: Some(H256::random()),
                transactions: vec![tx_hash],
                ..Default::default()
            }
            .into();

            db_client
                .insert_block_data(&[dummy_block], &[dummy_transaction])
                .await
                .unwrap();
        }

        func(db_client).await
    })
    .await
}

#[tokio::test]
async fn test_get_blocks() {
    with_filled_db(|db_client| async {
        let (port, handle) = new_server(db_client).await;

        let http_client =
            EthJsonRpcClient::new(ReqwestClient::new(format!("http://127.0.0.1:{port}")));

        let block_count = http_client.get_block_number().await.unwrap();
        assert_eq!(block_count, BLOCK_COUNT - 1);
        for i in 0u64..BLOCK_COUNT {
            let block = http_client
                .get_block_by_number(BlockNumber::Number(i.into()))
                .await
                .unwrap();
            assert_eq!(block.number, Some(i.into()));
            assert_eq!(block.transactions.len(), 1);

            let full_block = http_client
                .get_full_block_by_number(BlockNumber::Number(i.into()))
                .await
                .unwrap();
            assert_eq!(full_block.number, Some(i.into()));
            assert_eq!(full_block.transactions.len(), 1);
            assert_eq!(full_block.transactions[0].hash, block.transactions[0]);
        }

        {
            handle.stop().unwrap();
            handle.stopped().await;
        }
    })
    .await
}

#[tokio::test]
async fn test_get_blocks_rlp() {
    with_filled_db(|db_client| async {
        let (port, handle) = new_server(db_client).await;

        let http_client = ReqwestClient::new(format!("http://127.0.0.1:{port}"));
        // Test first five blocks
        let request = Request::Single(Call::MethodCall(MethodCall {
            jsonrpc: Some(Version::V2),
            method: "ic_getBlocksRLP".to_string(),
            params: Params::Array(vec![json!("0x0"), json!("0x5")]),
            id: Id::Str("ic_getBlocksRLP".to_string()),
        }));

        let Response::Single(Output::Success(result)) =
            http_client.send_rpc_request(request).await.unwrap()
        else {
            panic!("unexpected return type")
        };

        let data: String = serde_json::from_value(result.result).unwrap();
        let blocks: Vec<did::Block<did::Transaction>> =
            ethers_core::utils::rlp::decode_list(&hex::decode(data).unwrap());
        assert_eq!(blocks.len(), 5);
        assert_eq!(blocks[0].number, 0u64.into());
        assert_eq!(blocks[4].number, 4u64.into());

        // Test last 2 blocks
        let request = Request::Single(Call::MethodCall(MethodCall {
            jsonrpc: Some(Version::V2),
            method: "ic_getBlocksRLP".to_string(),
            params: Params::Array(vec![json!("0x8"), json!("0x5")]),
            id: Id::Str("ic_getBlocksRLP".to_string()),
        }));

        let Response::Single(Output::Success(result)) =
            http_client.send_rpc_request(request).await.unwrap()
        else {
            panic!("unexpected return type")
        };

        let data: String = serde_json::from_value(result.result).unwrap();
        let blocks: Vec<did::Block<did::Transaction>> =
            ethers_core::utils::rlp::decode_list(&hex::decode(data).unwrap());
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].number, 8u64.into());
        assert_eq!(blocks[1].number, 9u64.into());

        {
            handle.stop().unwrap();
            handle.stopped().await;
        }
    })
    .await
}

#[tokio::test]
async fn test_batched_request() {
    with_filled_db(|db_client| async {
        let (port, handle) = new_server(db_client).await;

        let http_client = ReqwestClient::new(format!("http://127.0.0.1:{port}"));
        let request = Request::Batch(vec![
            Call::MethodCall(MethodCall {
                jsonrpc: Some(Version::V2),
                method: "ic_getBlocksRLP".to_string(),
                params: Params::Array(vec![json!("0x0"), json!("0x5")]),
                id: Id::Str("ic_getBlocksRLP".to_string()),
            }),
            Call::MethodCall(MethodCall {
                jsonrpc: Some(Version::V2),
                method: "eth_blockNumber".to_string(),
                params: Params::Array(vec![]),
                id: Id::Str("eth_blockNumber".to_string()),
            }),
        ]);

        let Response::Batch(results) = http_client.send_rpc_request(request).await.unwrap() else {
            panic!("unexpected return type")
        };

        match &results[..] {
            [Output::Success(result_1), Output::Success(result_2)] => {
                assert_eq!(result_1.id, Id::Str("ic_getBlocksRLP".to_string()));
                let data: String = serde_json::from_value(result_1.result.clone()).unwrap();
                let blocks: Vec<did::Block<did::Transaction>> =
                    ethers_core::utils::rlp::decode_list(&hex::decode(data).unwrap());
                assert_eq!(blocks.len(), 5);

                assert_eq!(result_2.id, Id::Str("eth_blockNumber".to_string()));
                let data: String = serde_json::from_value(result_2.result.clone()).unwrap();
                let result = u64::from_str_radix(data.trim_start_matches("0x"), 16).unwrap();
                assert_eq!(result, BLOCK_COUNT - 1);
            }
            _ => panic!("unexpected results"),
        }

        {
            handle.stop().unwrap();
            handle.stopped().await;
        }
    })
    .await
}

#[tokio::test]
async fn test_get_genesis_accounts() {
    test_with_clients(|db_client| async move {
        // Arrange
        db_client.init(None, false).await.unwrap();

        let (port, handle) = new_server(db_client.clone()).await;

        let http_client =
            EthJsonRpcClient::new(ReqwestClient::new(format!("http://127.0.0.1:{port}")));

        // Test on empty database
        {
            // Act
            let genesis_accounts = http_client.get_genesis_balances().await.unwrap();

            // Assert
            assert!(genesis_accounts.is_empty());
        }

        // Test with existing genesis accounts
        {
            // Arrange
            let genesis_balances = vec![
                AccountBalance {
                    address: H160::from(ethers_core::types::H160::random()),
                    balance: U256::from(100_u64),
                },
                AccountBalance {
                    address: H160::from(ethers_core::types::H160::random()),
                    balance: U256::from(200_u64),
                },
            ];

            // Act
            db_client
                .insert_genesis_balances(&genesis_balances)
                .await
                .unwrap();

            let genesis_accounts = http_client.get_genesis_balances().await.unwrap();
            let genesis_accounts: Vec<AccountBalance> = genesis_accounts
                .into_iter()
                .map(|account| AccountBalance {
                    address: account.0.into(),
                    balance: account.1.into(),
                })
                .collect();

            // Assert
            assert_eq!(genesis_accounts, genesis_balances);
        }

        {
            handle.stop().unwrap();
            handle.stopped().await;
        }
    })
    .await
}

#[tokio::test]
async fn test_get_chain_id() {
    test_with_clients(|db_client| async move {
        // Arrange
        db_client.init(None, false).await.unwrap();

        let (port, handle) = new_server(db_client.clone()).await;

        let http_client =
            EthJsonRpcClient::new(ReqwestClient::new(format!("http://127.0.0.1:{port}")));

        let chain_id: u64 = random();
        db_client.insert_chain_id(chain_id).await.unwrap();

        // Act
        let chain_id = http_client.get_chain_id().await.unwrap();

        // Assert
        assert!(chain_id > 0);

        {
            handle.stop().unwrap();
            handle.stopped().await;
        }
    })
    .await
}

#[tokio::test]
async fn test_get_block_by_number_variants() {
    with_filled_db(|db_client| async {
        let (port, handle) = new_server(db_client).await;

        let http_client = ReqwestClient::new(format!("http://127.0.0.1:{port}"));
        let request = Request::Batch(vec![
            Call::MethodCall(MethodCall {
                jsonrpc: Some(Version::V2),
                method: "eth_getBlockByNumber".to_string(),
                params: Params::Array(vec![json!("latest"), json!(false)]),
                id: Id::Str("eth_getBlockByNumber".to_string()),
            }),
            Call::MethodCall(MethodCall {
                jsonrpc: Some(Version::V2),
                method: "eth_getBlockByNumber".to_string(),
                params: Params::Array(vec![json!("earliest"), json!(false)]),
                id: Id::Str("eth_getBlockByNumber".to_string()),
            }),
            Call::MethodCall(MethodCall {
                jsonrpc: Some(Version::V2),
                method: "eth_getBlockByNumber".to_string(),
                params: Params::Array(vec![json!("0x5"), json!(false)]),
                id: Id::Str("eth_getBlockByNumber".to_string()),
            }),

        ]);

        let Response::Batch(results) = http_client.send_rpc_request(request).await.unwrap() else {
            panic!("unexpected return type")
        };

        match &results[..] {
            [Output::Success(latest_block), Output::Success(earliest_block), Output::Success(number_block)] => {
                assert_eq!(latest_block.id, Id::Str("eth_getBlockByNumber".to_string()));
                let latest_block: Block<H256> =
                    serde_json::from_value(latest_block.result.clone()).unwrap();
                assert_eq!(latest_block.number, U64::from(BLOCK_COUNT - 1));

                let earliest_block: Block<H256> =
                    serde_json::from_value(earliest_block.result.clone()).unwrap();
                assert_eq!(earliest_block.number, U64::zero());

                let number_block: Block<H256> =
                    serde_json::from_value(number_block.result.clone()).unwrap();
                    assert_eq!(number_block.number, U64::from_hex_str("0x5").unwrap());
            }
            _ => panic!("unexpected results"),
        }

        // stop the server
        {
            handle.stop().unwrap();
            handle.stopped().await;
        }
    })
    .await
}

async fn new_server(db_client: Arc<dyn DatabaseClient>) -> (u16, ServerHandle) {
    let eth = EthImpl::new(db_client);
    let mut module = RpcModule::new(());
    module.merge(EthServer::into_rpc(eth.clone())).unwrap();
    module.merge(ICServer::into_rpc(eth)).unwrap();

    loop {
        let port = port_check::free_local_port().unwrap();
        if let Ok(server) = Server::builder().build(format!("0.0.0.0:{port}")).await {
            return (port, server.start(module));
        }
    }
}
