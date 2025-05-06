use std::sync::Arc;

use alloy::primitives::{Address, B256};
use did::evm_state::EvmGlobalState;
use did::rpc::id::Id;
use did::rpc::params::Params;
use did::rpc::request::{Request, RpcRequest};
use did::rpc::response::{Response, RpcResponse};
use did::rpc::version::Version;
use did::{Block, BlockNumber, H160, H256, U64, U256};
use ethereum_json_rpc_client::reqwest::ReqwestClient;
use ethereum_json_rpc_client::{Client, EthJsonRpcClient};
use evm_block_extractor::database::postgres_db_client::PostgresDbClient;
use evm_block_extractor::database::{AccountBalance, CertifiedBlock, DatabaseClient};
use evm_block_extractor::rpc::{EthImpl, EthServer, ICServer};
use jsonrpsee::RpcModule;
use jsonrpsee::server::{Server, ServerHandle};
use rand::random;
use serde_json::json;

use crate::test_with_clients;
use crate::tests::block_extractor_it::MockClient;

const BLOCK_COUNT: u64 = 10;

async fn with_filled_db<Func: AsyncFn(Arc<PostgresDbClient>) -> ()>(func: Func) {
    test_with_clients(async |db_client| {
        db_client.init(None, false).await.unwrap();

        for i in 0..BLOCK_COUNT {
            let tx_hash = H256::from(B256::random());
            let dummy_transaction = did::Transaction {
                hash: tx_hash.clone(),
                block_number: Some(i.into()),
                ..Default::default()
            };
            let dummy_block = Block::<H256> {
                number: U64::from(i),
                hash: H256::from(B256::random()),
                transactions: vec![tx_hash],
                ..Default::default()
            };

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
        let (http_client, _port, handle) = new_server(db_client, None).await;

        let block_count = http_client.get_block_number().await.unwrap();
        assert_eq!(block_count, BLOCK_COUNT - 1);
        for i in 0u64..BLOCK_COUNT {
            let block = http_client
                .get_block_by_number(BlockNumber::Number(i.into()))
                .await
                .unwrap();
            assert_eq!(block.number, i.into());
            assert_eq!(block.transactions.len(), 1);

            let full_block = http_client
                .get_full_block_by_number(BlockNumber::Number(i.into()))
                .await
                .unwrap();
            assert_eq!(full_block.number, i.into());
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
async fn test_batched_request() {
    with_filled_db(|db_client| async {
        let (_http_client, port, handle) = new_server(db_client, None).await;

        let http_client = ReqwestClient::new(format!("http://127.0.0.1:{port}"));
        let request = RpcRequest::Batch(vec![
            Request {
                jsonrpc: Some(Version::V2),
                method: "ic_getGenesisBalances".to_string(),
                params: Params::Array(vec![]),
                id: Id::String("ic_getGenesisBalances".to_string()),
            },
            Request {
                jsonrpc: Some(Version::V2),
                method: "eth_blockNumber".to_string(),
                params: Params::Array(vec![]),
                id: Id::String("eth_blockNumber".to_string()),
            },
        ]);

        let RpcResponse::Batch(results) = http_client.send_rpc_request(request).await.unwrap()
        else {
            panic!("unexpected return type")
        };

        match &results[..] {
            [Response::Success(result_1), Response::Success(result_2)] => {
                assert_eq!(result_1.id, Id::String("ic_getGenesisBalances".to_string()));
                let data = serde_json::from_value::<Vec<(H160, U256)>>(result_1.result.clone());
                assert!(data.is_ok());

                assert_eq!(result_2.id, Id::String("eth_blockNumber".to_string()));
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
    test_with_clients(async move |db_client| {
        // Arrange
        db_client.init(None, false).await.unwrap();

        let (http_client, _port, handle) = new_server(db_client.clone(), None).await;

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
                    address: H160::from(Address::random()),
                    balance: U256::from(100_u64),
                },
                AccountBalance {
                    address: H160::from(Address::random()),
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
                    address: account.0,
                    balance: account.1,
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
    test_with_clients(async move |db_client| {
        // Arrange
        db_client.init(None, false).await.unwrap();

        let (http_client, _port, handle) = new_server(db_client.clone(), None).await;

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
        let (_http_client, port, handle) = new_server(db_client, None).await;

        let http_client = ReqwestClient::new(format!("http://127.0.0.1:{port}"));
        let request = RpcRequest::Batch(vec![
            Request {
                jsonrpc: Some(Version::V2),
                method: "eth_getBlockByNumber".to_string(),
                params: Params::Array(vec![json!("latest"), json!(false)]),
                id: Id::String("eth_getBlockByNumber".to_string()),
            },
            Request {
                jsonrpc: Some(Version::V2),
                method: "eth_getBlockByNumber".to_string(),
                params: Params::Array(vec![json!("earliest"), json!(false)]),
                id: Id::String("eth_getBlockByNumber".to_string()),
            },
            Request {
                jsonrpc: Some(Version::V2),
                method: "eth_getBlockByNumber".to_string(),
                params: Params::Array(vec![json!("0x5"), json!(false)]),
                id: Id::String("eth_getBlockByNumber".to_string()),
            },
        ]);

        let RpcResponse::Batch(results) = http_client.send_rpc_request(request).await.unwrap()
        else {
            panic!("unexpected return type")
        };

        match &results[..] {
            [
                Response::Success(latest_block),
                Response::Success(earliest_block),
                Response::Success(number_block),
            ] => {
                assert_eq!(
                    latest_block.id,
                    Id::String("eth_getBlockByNumber".to_string())
                );
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

#[tokio::test]
async fn test_get_last_certified_block() {
    test_with_clients(async move |db_client| {
        // Arrange
        db_client.init(None, false).await.unwrap();

        let block = Block::<did::H256> {
            number: 1u64.into(),
            ..Default::default()
        };

        let certified_block = CertifiedBlock {
            certificate: vec![1, 2, 3],
            witness: vec![5, 6, 7],
            data: block.clone(),
        };

        db_client
            .insert_certified_block_data(certified_block.clone())
            .await
            .unwrap();

        let (http_client, _port, handle) = new_server(db_client.clone(), None).await;

        // Act
        let certified_block = http_client.get_last_certified_block().await.unwrap();

        // Assert
        assert_eq!(certified_block.certificate, vec![1, 2, 3]);
        assert_eq!(certified_block.witness, vec![5, 6, 7]);
        assert_eq!(certified_block.data, block);

        {
            handle.stop().unwrap();
            handle.stopped().await;
        }
    })
    .await
}

#[tokio::test]
async fn test_get_evm_global_state() {
    with_filled_db(|db_client| async {
        let expected_state = EvmGlobalState::Staging {
            max_block_number: Some(random()),
        };

        // Create mock EVM client that will return a predefined state
        let mock_evm_client = Arc::new(EthJsonRpcClient::new(MockClient::new(
            expected_state.clone(),
        )));

        let (http_client, _port, handle) = new_server(db_client, Some(mock_evm_client)).await;

        // Call get_evm_global_state and verify we get back the expected state
        let state = http_client.get_evm_global_state().await.unwrap();
        assert_eq!(state, expected_state);

        // Cleanup
        handle.stop().unwrap();
        handle.stopped().await;
    })
    .await
}

async fn new_server(
    db_client: Arc<PostgresDbClient>,
    evm_client: Option<Arc<EthJsonRpcClient<MockClient>>>,
) -> (EthJsonRpcClient<ReqwestClient>, u16, ServerHandle) {
    let evm_client = evm_client.unwrap_or_else(|| {
        Arc::new(EthJsonRpcClient::new(MockClient::new(
            EvmGlobalState::Enabled,
        )))
    });

    let eth = EthImpl::<MockClient, PostgresDbClient>::new(db_client, evm_client);
    let mut module = RpcModule::new(());
    module.merge(EthServer::into_rpc(eth.clone())).unwrap();
    module.merge(ICServer::into_rpc(eth)).unwrap();

    loop {
        let port = port_check::free_local_port().unwrap();
        if let Ok(server) = Server::builder().build(format!("0.0.0.0:{port}")).await {
            let client =
                EthJsonRpcClient::new(ReqwestClient::new(format!("http://127.0.0.1:{port}")));
            return (client, port, server.start(module));
        }
    }
}
