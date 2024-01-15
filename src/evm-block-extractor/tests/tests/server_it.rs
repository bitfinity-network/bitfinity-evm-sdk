use ethereum_json_rpc_client::{
    reqwest::{self, ReqwestClient},
    Client, EthJsonRcpClient,
};
use ethers_core::types::{Block, BlockNumber, Transaction, TransactionReceipt, H256};
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
        db_client.init().await.unwrap();

        let mut blocks = Vec::new();
        let mut receipts = Vec::new();

        for i in 0..BLOCK_COUNT {
            let tx_hash = H256::random();
            let dummy_block: Block<Transaction> = ethers_core::types::Block {
                number: Some(ethers_core::types::U64::from(i)),
                hash: Some(H256::random()),
                transactions: vec![Transaction {
                    hash: tx_hash.clone(),
                    ..Default::default()
                }],
                ..Default::default()
            };
            let dummy_receipt: TransactionReceipt = ethers_core::types::TransactionReceipt {
                transaction_hash: tx_hash,
                block_number: Some(i.into()),
                block_hash: dummy_block.hash.clone(),
                ..Default::default()
            };

            blocks.push(dummy_block);

            receipts.push(dummy_receipt);
        }

        db_client
            .insert_blocks_and_receipts(&blocks, &receipts)
            .await
            .unwrap();

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

        let server = Server::builder().build("0.0.0.0:9001").await.unwrap();
        let handle = server.start(module);

        let http_client =
            EthJsonRcpClient::new(ReqwestClient::new("http://127.0.0.2:9001".to_string()));

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

        let server = Server::builder().build("0.0.0.0:9002").await.unwrap();
        let handle = server.start(module);

        let http_client = ReqwestClient::new("http://127.0.0.2:9002".to_string());

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
