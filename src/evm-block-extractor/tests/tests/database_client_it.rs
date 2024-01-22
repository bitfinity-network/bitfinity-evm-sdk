use did::block::ExeResult;
use did::transaction::StorableExecutionResult;
use did::{Block, Transaction, H256, U256, U64};

use crate::test_with_clients;

#[tokio::test]
async fn test_batch_insertion_of_blocks_and_receipts_transactions_retrieval() {
    test_with_clients(|db_client| async move {
        db_client.init().await.unwrap();

        let mut blocks = Vec::new();

        for i in 1..=10 {
            let dummy_block: Block<H256> = Block {
                number: ethers_core::types::U64::from(i).into(),
                hash: ethers_core::types::H256::random().into(),
                ..Default::default()
            };

            blocks.push(dummy_block);
        }

        let mut exe_results = Vec::new();

        for i in 1..=10 {
            let tx_hash = ethers_core::types::H256::random();
            let dummy_exe_result: StorableExecutionResult = StorableExecutionResult {
                transaction_hash: tx_hash.into(),
                block_hash: blocks[i - 1].hash.clone(),
                exe_result: ExeResult::success(
                    U256::max_value(),
                    did::block::TransactOut::None,
                    vec![],
                ),
                transaction_index: Default::default(),
                block_number: U64::from(i),
                from: Default::default(),
                to: Default::default(),
                transaction_type: Default::default(),
                cumulative_gas_used: Default::default(),
                max_fee_per_gas: Default::default(),
                gas_price: Default::default(),
                max_priority_fee_per_gas: Default::default(),
            };

            exe_results.push(dummy_exe_result);
        }

        let mut txn = vec![];
        for i in 0..10 {
            let tx_hash = &exe_results[i].transaction_hash;
            let block_number = blocks[i].number.0.as_u64();
            let dummy_txn = Transaction {
                hash: tx_hash.clone().into(),
                block_number: Some(U64::from(block_number)),
                ..Default::default()
            };

            txn.push(dummy_txn);
        }

        db_client
            .insert_block_data(&blocks, &exe_results, &txn)
            .await
            .unwrap();

        let block = db_client.get_full_block_by_number(1).await.unwrap();

        // Check the transactions
        assert_eq!(block.transactions.len(), 1);
        assert_eq!(block.transactions[0].hash, exe_results[0].transaction_hash);

        assert_eq!(block.number.0.as_u64(), 1);

        let receipt = db_client
            .get_transaction_receipt(exe_results[0].transaction_hash.clone())
            .await
            .unwrap();

        assert_eq!(
            receipt.transaction_hash,
            exe_results[0].transaction_hash.clone()
        );

        let block = db_client.get_full_block_by_number(10).await.unwrap();

        assert_eq!(block.number.0.as_u64(), 10);

        let receipt = db_client
            .get_transaction_receipt(exe_results[9].transaction_hash.clone())
            .await
            .unwrap();

        assert_eq!(receipt.transaction_hash, exe_results[9].transaction_hash);
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
            let dummy_block: Block<H256> = Block {
                number: ethers_core::types::U64::from(i).into(),
                hash: ethers_core::types::H256::random().into(),
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
        let dummy_block: Block<H256> = Block {
            number: ethers_core::types::U64::from(1).into(),
            hash: ethers_core::types::H256::random().into(),
            ..Default::default()
        };

        assert!(db_client
            .insert_block_data(&[dummy_block], &[], &[])
            .await
            .is_err());

        // First initialization - creates tables
        db_client.init().await.unwrap();

        // Add a block
        let dummy_block: Block<H256> = Block {
            number: ethers_core::types::U64::from(1).into(),
            hash: ethers_core::types::H256::random().into(),
            ..Default::default()
        };

        assert!(db_client
            .insert_block_data(&[dummy_block], &[], &[])
            .await
            .is_ok());

        assert!(db_client.init().await.is_ok());

        // Retrieve the block
        let block = db_client.get_block_by_number(1).await.unwrap();

        assert_eq!(block.number.0.as_u64(), 1);
    })
    .await;
}

#[tokio::test]
async fn test_retrieval_of_transactions_with_blocks() {
    test_with_clients(|db_client| async move {
        db_client.init().await.unwrap();

        let mut blocks = Vec::new();

        for i in 1..=10 {
            let dummy_block: Block<H256> = Block {
                number: ethers_core::types::U64::from(i).into(),
                hash: ethers_core::types::H256::random().into(),
                ..Default::default()
            };

            blocks.push(dummy_block);
        }

        let mut txn = vec![];
        for _ in 0..10 {
            let dummy_txn = Transaction {
                hash: ethers_core::types::H256::random().into(),
                block_number: Some(5_u64.into()),
                block_hash: Some(blocks[4].hash.clone()),
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

        assert_eq!(block.number.0.as_u64(), 1);

        let block = db_client.get_full_block_by_number(5).await.unwrap();

        assert_eq!(block.hash, blocks[4].hash);

        assert_eq!(block.number.0.as_u64(), 5);
        assert_eq!(block.transactions.len(), 10);

        for txn in block.transactions {
            assert_eq!(txn.block_number.unwrap().0.as_u64(), 5);
            assert_eq!(txn.block_hash.unwrap(), block.hash);
        }
    })
    .await;
}
