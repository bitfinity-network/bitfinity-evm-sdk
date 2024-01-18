use std::sync::Arc;

use ethereum_json_rpc_client::reqwest::ReqwestClient;
use ethereum_json_rpc_client::EthJsonRcpClient;
use ethers_core::types::{Block, H256};
use evm_block_extractor::block_extractor::BlockExtractor;

use crate::test_with_clients;

#[tokio::test]
async fn test_extractor_collect_blocks() {
    test_with_clients(|db_client| async move {
        db_client.init().await.unwrap();

        let rpc_url = "https://testnet.bitfinity.network".to_string();
        let evm_client = Arc::new(EthJsonRcpClient::new(ReqwestClient::new(rpc_url)));

        let request_time_out_secs = 10;
        let rpc_batch_size = 50;
        let mut extractor = BlockExtractor::new(
            evm_client.clone(),
            request_time_out_secs,
            rpc_batch_size,
            db_client.clone(),
        );

        let end_block = evm_client.get_block_number().await.unwrap();
        let start_block = end_block - 10;

        println!("Getting blocks from {:?} to {}", start_block, end_block);

        let result = extractor
            .collect_blocks(start_block, end_block)
            .await
            .unwrap();

        assert_eq!(result.0, start_block);
        assert_eq!(result.1, end_block);

        let latest_block_num: Block<H256> = db_client
                .get_block_by_number(end_block)
                .await
                .unwrap();

        assert_eq!(end_block, latest_block_num.number.unwrap().as_u64());
    })
    .await;
}
