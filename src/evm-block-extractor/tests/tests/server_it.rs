use did::{
    block::{ExeResult, TransactOut},
    transaction::{Bloom, StorableExecutionResult},
    H160,
};
use ethereum_json_rpc_client::{reqwest::ReqwestClient, Client, EthJsonRcpClient};
use ethers_core::types::{BlockNumber, Transaction, H256};
use evm_block_extractor::{
    database::DatabaseClient,
    rpc::{EthImpl, EthServer, ICServer},
};
use jsonrpc_core::{Call, Id, MethodCall, Output, Params, Request, Response, Version};
use jsonrpsee::{server::Server, RpcModule};
use serde_json::json;
use std::future::Future;
use std::sync::Arc;

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
            let dummy_receipt = StorableExecutionResult {
                transaction_hash: tx_hash.into(),
                block_number: i.into(),
                block_hash: dummy_block.hash.clone(),
                exe_result: ExeResult::Success {
                    gas_used: (i * 1000).into(),
                    logs: vec![],
                    logs_bloom: Box::new(Bloom::zeros()),
                    output: TransactOut::None,
                },
                transaction_index: 0u64.into(),
                from: H160::default(),
                to: None,
                transaction_type: None,
                cumulative_gas_used: (i * 1000).into(),
                max_fee_per_gas: None,
                gas_price: None,
                max_priority_fee_per_gas: None,
            };

            db_client
                .insert_block_data(&[dummy_block], &[dummy_receipt], &[dummy_transaction])
                .await
                .unwrap();
        }

        func(db_client).await
    })
    .await
}

#[tokio::test]
async fn test_get_blocks_and_receipts() {
    with_filled_db(|db_client| async {
        let eth = EthImpl::new(db_client);
        let mut module = RpcModule::new(());
        module.merge(EthServer::into_rpc(eth.clone())).unwrap();
        module.merge(ICServer::into_rpc(eth)).unwrap();

        let port = port_check::free_local_port_in_range(9000, 9099).unwrap();
        let server = Server::builder()
            .build(format!("0.0.0.0:{port}"))
            .await
            .unwrap();
        let handle = server.start(module);

        let http_client =
            EthJsonRcpClient::new(ReqwestClient::new(format!("http://127.0.0.1:{port}")));

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

            let receipt = http_client
                .get_receipt_by_hash(block.transactions[0])
                .await
                .unwrap();
            assert_eq!(receipt.block_number, Some(i.into()));
            assert_eq!(receipt.block_hash, block.hash);
            assert_eq!(receipt.transaction_hash, block.transactions[0]);
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
        let eth = EthImpl::new(db_client);
        let mut module = RpcModule::new(());
        module.merge(EthServer::into_rpc(eth.clone())).unwrap();
        module.merge(ICServer::into_rpc(eth)).unwrap();

        let port = port_check::free_local_port_in_range(9100, 9199).unwrap();
        let server = Server::builder()
            .build(format!("0.0.0.0:{port}"))
            .await
            .unwrap();
        let handle = server.start(module);

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
        let eth = EthImpl::new(db_client);
        let mut module = RpcModule::new(());
        module.merge(EthServer::into_rpc(eth.clone())).unwrap();
        module.merge(ICServer::into_rpc(eth)).unwrap();

        let port = port_check::free_local_port_in_range(9200, 9299).unwrap();
        let server = Server::builder()
            .build(format!("0.0.0.0:{port}"))
            .await
            .unwrap();
        let handle = server.start(module);

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
