use std::sync::Arc;

use ethereum_json_rpc_client::EthJsonRcpClient;
use ethereum_json_rpc_client::reqwest::ReqwestClient;
use evm_block_extractor::block_extractor::BlockExtractor;
use evm_block_extractor::database::big_query_db_client::BigQueryDbClient;
use evm_block_extractor::database::DatabaseClient;
use testcontainers::testcontainers::clients::Cli;

mod client;

#[tokio::test]
async fn test_extractor_collect_blocks() {
    let docker = Cli::default();
    let project_id = format!("test_project_{}", rand::random::<u64>());
    let (gcp_client, _node, _temp_file, _auth) =
        client::new_bigquery_client(&docker, &project_id).await;
    let dataset_id = format!("test_{}", rand::random::<u64>());

    let blockchain = Arc::new(
        BigQueryDbClient::new_with_client(
            project_id.clone(),
            dataset_id.clone(),
            gcp_client.clone(),
        )
        .unwrap(),
    );

    blockchain.init().await.unwrap();

    let rpc_url = "https://testnet.bitfinity.network".to_string();
    let evm_client = Arc::new(EthJsonRcpClient::new(ReqwestClient::new(
        rpc_url,
    )));

    let request_time_out_secs = 10;
    let rpc_batch_size = 50;
    let mut extractor = BlockExtractor::new(
        evm_client.clone(),
        request_time_out_secs,
        rpc_batch_size,
        blockchain.clone(),
    );

    let end_block = evm_client.get_block_number().await.unwrap();
    let start_block = end_block - 10;

    println!("Getting blocks from {:?} to {}", start_block, end_block);

    let result = extractor.collect_blocks(start_block, end_block).await.unwrap();

    assert_eq!(result.0, start_block);
    assert_eq!(result.1, end_block);

    let latest_block_num = blockchain
        .get_block_by_number(end_block)
        .await
        .unwrap()
        .number
        .unwrap();

    assert_eq!(end_block, latest_block_num.as_u64());
}
