use ethers_core::types::{Block, Transaction, TransactionReceipt, H256};
use testcontainers::testcontainers::clients::Cli;

use crate::test_with_clients;

#[tokio::test]
async fn test_batch_insertion_of_blocks_and_receipts_retrieval() {
    test_with_clients(|db_client| async move {
        db_client.init(None).await.unwrap();

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

        db_client
            .insert_blocks_and_receipts(&blocks, &receipts)
            .await
            .unwrap();

        let block = db_client.get_block_by_number(1).await.unwrap();

        assert_eq!(block.number.unwrap().as_u64(), 1);

        let receipt = db_client
            .get_transaction_receipt(receipts[0].transaction_hash)
            .await
            .unwrap();

        assert_eq!(receipt.transaction_hash, receipts[0].transaction_hash);

        let block = db_client.get_block_by_number(10).await.unwrap();

        assert_eq!(block.number.unwrap().as_u64(), 10);

        let receipt = db_client
            .get_transaction_receipt(receipts[9].transaction_hash)
            .await
            .unwrap();

        assert_eq!(receipt.transaction_hash, receipts[9].transaction_hash);
    })
    .await;
}

#[tokio::test]
async fn test_retrieval_of_latest_and_oldest_block_number() {
    test_with_clients(|db_client| async move {
        db_client.init(None).await.unwrap();

        let latest_block_number = db_client.get_latest_block_number().await.unwrap();
        assert!(latest_block_number.is_none());

        for i in 1..=10 {
            let dummy_block: Block<Transaction> = ethers_core::types::Block {
                number: Some(ethers_core::types::U64::from(i)),
                hash: Some(H256::random()),
                ..Default::default()
            };

            db_client
                .insert_blocks_and_receipts(&[dummy_block], &[])
                .await
                .unwrap();
        }

        let latest_block_number = db_client.get_latest_block_number().await.unwrap();
        assert_eq!(latest_block_number, Some(10));

        let earliest_block_number = db_client.get_earliest_block_number().await.unwrap();
        assert_eq!(earliest_block_number, 1);
    })
    .await;
}

#[tokio::test]
async fn test_init_idempotency() {
    test_with_clients(|db_client| async move {
        // Add a block
        let dummy_block: Block<Transaction> = ethers_core::types::Block {
            number: Some(ethers_core::types::U64::from(1)),
            hash: Some(H256::random()),
            ..Default::default()
        };

        assert!(db_client
            .insert_blocks_and_receipts(&[dummy_block], &[])
            .await
            .is_err());

        // First initialization - creates tables
        db_client.init(None).await.unwrap();

        // Add a block
        let dummy_block: Block<Transaction> = ethers_core::types::Block {
            number: Some(ethers_core::types::U64::from(1)),
            hash: Some(H256::random()),
            ..Default::default()
        };

        assert!(db_client
            .insert_blocks_and_receipts(&[dummy_block], &[])
            .await
            .is_ok());

        assert!(db_client.init(None).await.is_ok());

        // Retrieve the block
        let block = db_client.get_block_by_number(1).await.unwrap();

        assert_eq!(block.number.unwrap().as_u64(), 1);
    })
    .await;
}

#[tokio::test]
async fn test_deletion_and_creation_of_table_when_earliest_block_are_different() {
    let docker = Cli::default();
    let (bigquery_client, _node, _temp_file, _auth) =
        crate::client::new_bigquery_client(&docker).await;

    let genesis1: Block<Transaction> = ethers_core::types::Block {
        number: Some(ethers_core::types::U64::from(0)),
        hash: Some(H256::random()),
        ..Default::default()
    };

    let genesis2: Block<Transaction> = ethers_core::types::Block {
        number: Some(ethers_core::types::U64::from(0)),
        hash: Some(H256::random()),
        ..Default::default()
    };

    bigquery_client.init(None).await.unwrap();

    assert!(bigquery_client
        .insert_blocks_and_receipts(&[genesis1.clone()], &[])
        .await
        .is_ok());

    let block = bigquery_client.get_block_by_number(0).await.unwrap();

    assert_eq!(block.number.unwrap().as_u64(), 0);
    assert_eq!(block.hash.unwrap(), genesis1.hash.unwrap());

    // Init with genesis2
    bigquery_client
        .init(Some(genesis2.clone().into()))
        .await
        .unwrap();

    // check the database is empty
    let latest_block_number = bigquery_client.get_latest_block_number().await.unwrap();

    assert!(latest_block_number.is_none());

    // Add a block
    assert!(bigquery_client
        .insert_blocks_and_receipts(&[genesis2.clone()], &[])
        .await
        .is_ok());

    // Retrieve the block
    let block = bigquery_client.get_block_by_number(0).await.unwrap();

    assert_eq!(block.number.unwrap().as_u64(), 0);
    // hash should be genesis2
    assert_eq!(block.hash.unwrap(), genesis2.hash.unwrap());
}
