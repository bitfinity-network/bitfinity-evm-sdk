use evm_block_extractor::block_extractor::BlockExtractor;
use evm_block_extractor::storage_clients::gcp_big_query::BigQueryBlockChain;
use gcp_bigquery_client::model::dataset::Dataset;
use testcontainers::clients::Cli;

mod client;

#[tokio::test]
async fn test_extractor_collect_blocks() {
    let docker = Cli::default();
    let project_id = format!("test_project_{}", rand::random::<u64>());
    let (gcp_client, _node, _temp_file, _auth) =
        client::new_bigquery_client(&docker, &project_id).await;
    let dataset_id = format!("test_{}", rand::random::<u64>());

    let blockchain = Box::new(
        BigQueryBlockChain::new_with_client(
            project_id.clone(),
            dataset_id.clone(),
            gcp_client.clone(),
        )
        .unwrap(),
    );

    // Create dataset
    gcp_client
        .dataset()
        .create(Dataset::new(&project_id, &dataset_id))
        .await
        .unwrap();

    blockchain.init().await.unwrap();

    let rpc_url = "https://testnet.bitfinity.network".to_string();
    let request_time_out_secs = 10;
    let rpc_batch_size = 50;
    let mut extractor =
        BlockExtractor::new(rpc_url, request_time_out_secs, rpc_batch_size, blockchain);

    let end_block = extractor.latest_block_number().await.unwrap();
    let start_block = end_block - 10;
    let max_requests = 50;
    let block_range = start_block..=end_block;

    for block_number in block_range {
        println!("Processing block number: {}", block_number);
    }
    println!("Getting blocks from {} to {}", start_block, end_block);

    let result = extractor
        .collect_blocks(start_block..=end_block, max_requests)
        .await;

    if let Err(e) = &result {
        println!("Error: {:?}", e);
    }

    assert!(result.is_ok());

    let latest_block_num = extractor
        .blockchain
        .get_block_by_number(end_block)
        .await
        .unwrap()
        .number
        .unwrap();

    assert_eq!(end_block, latest_block_num.as_u64());
}
