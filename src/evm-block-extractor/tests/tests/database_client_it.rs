use ethers_core::types::{Block, Transaction, TransactionReceipt, H256};

use crate::test_with_clients;

#[tokio::test]
async fn test_batch_insertion_of_blocks_and_receipts_transactions_retrieval() {
    test_with_clients(|db_client| async move {
        db_client.init().await.unwrap();

        let mut blocks = Vec::new();

        for i in 1..=10 {
            let dummy_block: Block<H256> = ethers_core::types::Block {
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

        let mut txn = vec![];
        for i in 0..10 {
            let tx_hash = receipts[i].transaction_hash;
            let block_number = blocks[i].number.unwrap().as_u64();
            let dummy_txn: Transaction = ethers_core::types::Transaction {
                hash: tx_hash,
                block_number: Some(ethers_core::types::U64::from(block_number)),
                ..Default::default()
            };

            txn.push(dummy_txn);
        }

        db_client
            .insert_block_data(&blocks, &receipts, &txn)
            .await
            .unwrap();

        let block = db_client.get_full_block_by_number(1).await.unwrap();

        // Check the transactions
        assert_eq!(block.transactions.len(), 1);
        assert_eq!(block.transactions[0].hash, receipts[0].transaction_hash);

        assert_eq!(block.number.unwrap().as_u64(), 1);

        let receipt = db_client
            .get_transaction_receipt(receipts[0].transaction_hash)
            .await
            .unwrap();

        assert_eq!(receipt.transaction_hash, receipts[0].transaction_hash);

        let block = db_client.get_full_block_by_number(10).await.unwrap();

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
        db_client.init().await.unwrap();

        let latest_block_number = db_client.get_latest_block_number().await.unwrap();
        assert!(latest_block_number.is_none());

        for i in 1..=10 {
            let dummy_block: Block<H256> = ethers_core::types::Block {
                number: Some(ethers_core::types::U64::from(i)),
                hash: Some(H256::random()),
                ..Default::default()
            };

            db_client
                .insert_block_data(&[dummy_block], &[], &[])
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
        let dummy_block: Block<H256> = ethers_core::types::Block {
            number: Some(ethers_core::types::U64::from(1)),
            hash: Some(H256::random()),
            ..Default::default()
        };

        assert!(db_client
            .insert_block_data(&[dummy_block], &[], &[])
            .await
            .is_err());

        // First initialization - creates tables
        db_client.init().await.unwrap();

        // Add a block
        let dummy_block: Block<H256> = ethers_core::types::Block {
            number: Some(ethers_core::types::U64::from(1)),
            hash: Some(H256::random()),
            ..Default::default()
        };

        assert!(db_client
            .insert_block_data(&[dummy_block], &[], &[])
            .await
            .is_ok());

        assert!(db_client.init().await.is_ok());

        // Retrieve the block
        let block = db_client.get_block_by_number(1).await.unwrap();

        assert_eq!(block.number.unwrap().as_u64(), 1);
    })
    .await;
}

#[tokio::test]
async fn test_retrieval_of_transactions_with_blocks() {
    test_with_clients(|db_client| async move {
        db_client.init().await.unwrap();

        let mut blocks = Vec::new();

        for i in 1..=10 {
            let dummy_block: Block<H256> = ethers_core::types::Block {
                number: Some(ethers_core::types::U64::from(i)),
                hash: Some(H256::random()),
                ..Default::default()
            };

            blocks.push(dummy_block);
        }

        let mut txn = vec![];
        for _ in 0..10 {
            let dummy_txn: Transaction = ethers_core::types::Transaction {
                hash: H256::random(),
                block_number: Some(5_u64.into()),
                block_hash: Some(blocks[4].hash.unwrap()),
                ..Default::default()
            };

            txn.push(dummy_txn);
        }

        db_client
            .insert_block_data(&blocks, &[], &txn)
            .await
            .unwrap();

        let block = db_client.get_block_by_number(1).await.unwrap();

        // Check the transactions
        assert_eq!(block.transactions.len(), 0);

        assert_eq!(block.number.unwrap().as_u64(), 1);

        let block = db_client.get_full_block_by_number(5).await.unwrap();

        assert_eq!(block.hash.unwrap(), blocks[4].hash.unwrap());

        assert_eq!(block.number.unwrap().as_u64(), 5);
        assert_eq!(block.transactions.len(), 10);

        for txn in block.transactions {
            assert_eq!(txn.block_number.unwrap().as_u64(), 5);
            assert_eq!(txn.block_hash.unwrap(), block.hash.unwrap());
        }
    })
    .await;
}
