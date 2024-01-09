mod client;
use ethers_core::types::{Block, Transaction, TransactionReceipt, H256};
use evm_block_extractor::storage_clients::gcp_big_query::BigQueryBlockChain;
use evm_block_extractor::storage_clients::BlockChainDB;
use testcontainers::clients::Cli;

#[tokio::test]
async fn test_batch_insertion_of_blocks_and_receipts_retrieval_in_bq() {
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

    blockchain.init().await.unwrap();

    let mut blocks = Vec::new();

    for i in 1..=10 {
        let dummy_block: Block<Transaction> = ethers_core::types::Block {
            number: Some(ethers_core::types::U64::from(i)),
            hash: Some(H256::random()),
            ..Default::default()
        };

        blocks.push(dummy_block);
    }

    let mut receipts = Vec::new();

    for _ in 1..=10 {
        let tx_hash = H256::random();
        let dummy_receipt: TransactionReceipt = ethers_core::types::TransactionReceipt {
            transaction_hash: tx_hash,
            ..Default::default()
        };

        receipts.push(dummy_receipt);
    }

    blockchain
        .insert_blocks_and_receipts(&blocks, &receipts)
        .await
        .unwrap();

    let block = blockchain.get_block_by_number(1).await.unwrap();

    assert_eq!(block.number.unwrap().as_u64(), 1);

    let receipt = blockchain
        .get_transaction_receipt(receipts[0].transaction_hash)
        .await
        .unwrap();

    assert_eq!(receipt.transaction_hash, receipts[0].transaction_hash);

    let block = blockchain.get_block_by_number(10).await.unwrap();

    assert_eq!(block.number.unwrap().as_u64(), 10);

    let receipt = blockchain
        .get_transaction_receipt(receipts[9].transaction_hash)
        .await
        .unwrap();

    assert_eq!(receipt.transaction_hash, receipts[9].transaction_hash);
}

#[tokio::test]
async fn test_retrieval_of_latest_and_oldest_block_number() {
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

    blockchain.init().await.unwrap();

    for i in 1..=10 {
        let dummy_block: Block<Transaction> = ethers_core::types::Block {
            number: Some(ethers_core::types::U64::from(i)),
            hash: Some(H256::random()),
            ..Default::default()
        };

        blockchain
            .insert_blocks_and_receipts(&[dummy_block], &[])
            .await
            .unwrap();
    }

    let latest_block_number = blockchain.get_latest_block_number().await.unwrap();

    assert_eq!(latest_block_number, 10);

    let earliest_block_number = blockchain.get_earliest_block_number().await.unwrap();

    assert_eq!(earliest_block_number, 1);
}

#[tokio::test]
async fn test_init_idempotency() {
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

    // Add a block
    let dummy_block: Block<Transaction> = ethers_core::types::Block {
        number: Some(ethers_core::types::U64::from(1)),
        hash: Some(H256::random()),
        ..Default::default()
    };

    assert!(blockchain
        .insert_blocks_and_receipts(&[dummy_block], &[])
        .await
        .is_err());

    // First initialization - creates tables
    blockchain.init().await.unwrap();

    // Add a block
    let dummy_block: Block<Transaction> = ethers_core::types::Block {
        number: Some(ethers_core::types::U64::from(1)),
        hash: Some(H256::random()),
        ..Default::default()
    };

    assert!(blockchain
        .insert_blocks_and_receipts(&[dummy_block], &[])
        .await
        .is_ok());

    assert!(blockchain.init().await.is_ok());

    // Retrieve the block
    let block = blockchain.get_block_by_number(1).await.unwrap();

    assert_eq!(block.number.unwrap().as_u64(), 1);
}
