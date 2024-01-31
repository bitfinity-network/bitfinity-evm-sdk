use did::block::ExeResult;
use did::transaction::StorableExecutionResult;
use did::{Block, Transaction, H160, H256, U256, U64};
use evm_block_extractor::database::AccountBalance;

use crate::test_with_clients;

#[tokio::test]
async fn test_batch_insertion_of_blocks_and_receipts_transactions_retrieval() {
    test_with_clients(|db_client| async move {
        db_client.init(None, false).await.unwrap();

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
            let dummy_exe_result = new_storable_execution_result(
                tx_hash.into(),
                blocks[i - 1].hash.clone(),
                U64::from(i),
            );
            exe_results.push(dummy_exe_result);
        }

        let mut txn = vec![];
        for i in 0..10 {
            let tx_hash = &exe_results[i].transaction_hash;
            let block_number = blocks[i].number.0.as_u64();
            let dummy_txn = Transaction {
                hash: tx_hash.clone(),
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
        db_client.init(None, false).await.unwrap();

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
        db_client.init(None, false).await.unwrap();

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

        assert!(db_client.init(None, false).await.is_ok());

        // Retrieve the block
        let block = db_client.get_block_by_number(1).await.unwrap();

        assert_eq!(block.number.0.as_u64(), 1);
    })
    .await;
}

#[tokio::test]
async fn test_retrieval_of_transactions_with_blocks() {
    test_with_clients(|db_client| async move {
        db_client.init(None, false).await.unwrap();

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

#[tokio::test]
async fn test_deletion_and_creation_of_table_when_earliest_blocks_are_different() {
    test_with_clients(|db_client| async move {
        let block_one: Block<Transaction> = Block::<H256> {
            number: ethers_core::types::U64::from(0).into(),
            hash: ethers_core::types::H256::random().into(),
            ..Default::default()
        }
        .into_full_block(vec![]);

        let block_two: Block<Transaction> = Block::<H256> {
            number: ethers_core::types::U64::from(0).into(),
            hash: ethers_core::types::H256::random().into(),
            ..Default::default()
        }
        .into_full_block(vec![]);

        db_client.init(None, false).await.unwrap();

        assert!(db_client
            .insert_block_data(&[block_one.clone().into()], &[], &[])
            .await
            .is_ok());

        let block = db_client.get_block_by_number(0).await.unwrap();

        assert_eq!(block.number.0.as_u64(), 0);
        assert_eq!(block.hash, block_one.hash);

        // Init with block_two
        db_client
            .init(Some(block_two.clone().into()), true)
            .await
            .unwrap();

        // check the database is empty
        let latest_block_number = db_client.get_latest_block_number().await.unwrap();
        assert!(latest_block_number.is_none());

        // Add a block
        assert!(db_client
            .insert_block_data(&[block_two.clone().into()], &[], &[])
            .await
            .is_ok());

        // Retrieve the block
        let block = db_client.get_block_by_number(0).await.unwrap();

        assert_eq!(block.number.0.as_u64(), 0);
        // hash should be block_two's hash
        assert_eq!(block.hash, block_two.hash);
    })
    .await;
}

#[tokio::test]
async fn test_deletion_and_clearing_of_database() {
    test_with_clients(|db_client| async move {
        db_client.init(None, false).await.unwrap();

        db_client
            .insert_block_data(
                &[ethers_core::types::Block::<H256> {
                    number: Some(ethers_core::types::U64::zero()),
                    hash: Some(ethers_core::types::H256::random()),
                    ..Default::default()
                }
                .into()],
                &[new_storable_execution_result(
                    H256::zero(),
                    H256::zero(),
                    0_u64.into(),
                )],
                &[ethers_core::types::Transaction {
                    block_number: Some(ethers_core::types::U64::zero()),
                    hash: ethers_core::types::H256::random(),
                    ..Default::default()
                }
                .into()],
            )
            .await
            .unwrap();

        let block = db_client.get_block_by_number(0).await.unwrap();
        assert_eq!(block.number.0.as_u64(), 0);

        // Clear the database
        db_client.clear().await.unwrap();
    })
    .await;
}

#[tokio::test]
async fn test_database_reset_on_empty_db() {
    test_with_clients(|db_client| async move {
        // the first time init is called the DB has no tables
        db_client.init(None, true).await.unwrap();
        assert!(db_client.get_block_by_number(0).await.is_err());

        // the second time init is called the DB has empty tables
        db_client.init(None, true).await.unwrap();
        assert!(db_client.get_block_by_number(0).await.is_err());
    })
    .await;
}

#[tokio::test]
async fn test_check_if_same_block_hash() {
    test_with_clients(|db_client| async move {
        db_client.init(None, false).await.unwrap();

        let dummy_block: Block<H256> = Block {
            number: ethers_core::types::U64::from(1).into(),
            hash: ethers_core::types::H256::random().into(),
            ..Default::default()
        };

        db_client
            .insert_block_data(&[dummy_block.clone()], &[], &[])
            .await
            .unwrap();

        let same_block = db_client
            .check_if_same_block_hash(&dummy_block)
            .await
            .unwrap();

        assert!(same_block);

        let dummy_block: Block<H256> = Block {
            number: ethers_core::types::U64::from(1).into(),
            hash: ethers_core::types::H256::random().into(),
            ..Default::default()
        };

        let same_block = db_client
            .check_if_same_block_hash(&dummy_block)
            .await
            .unwrap();

        assert!(!same_block);
    })
    .await;
}

#[tokio::test]
async fn test_insertion_of_blocks_with_no_txs_and_no_receipts() {
    test_with_clients(|db_client| async move {
        db_client.init(None, false).await.unwrap();

        let dummy_block: Block<H256> = Block {
            number: ethers_core::types::U64::from(1).into(),
            hash: ethers_core::types::H256::random().into(),
            ..Default::default()
        };

        db_client
            .insert_block_data(&[dummy_block.clone()], &[], &[])
            .await
            .unwrap();

        let block = db_client.get_block_by_number(1).await.unwrap();

        assert_eq!(block.number.0.as_u64(), 1);
        assert_eq!(block.hash, dummy_block.hash);
    })
    .await;
}

#[tokio::test]
async fn test_insertion_of_blocks_with_txs_and_no_receipts() {
    test_with_clients(|db_client| async move {
        db_client.init(None, false).await.unwrap();

        let dummy_block: Block<H256> = Block {
            number: ethers_core::types::U64::from(1).into(),
            hash: ethers_core::types::H256::random().into(),
            ..Default::default()
        };

        let dummy_txn = Transaction {
            hash: ethers_core::types::H256::random().into(),
            block_number: Some(1_u64.into()),
            ..Default::default()
        };

        db_client
            .insert_block_data(&[dummy_block.clone()], &[], &[dummy_txn.clone()])
            .await
            .unwrap();

        let block = db_client.get_full_block_by_number(1).await.unwrap();

        assert_eq!(block.number.0.as_u64(), 1);
        assert_eq!(block.hash, dummy_block.hash);

        assert_eq!(block.transactions.len(), 1);
        assert_eq!(block.transactions[0].hash, dummy_txn.hash);
    })
    .await;
}

#[tokio::test]
async fn test_insertion_of_blocks_with_no_txs_and_receipts() {
    test_with_clients(|db_client| async move {
        db_client.init(None, false).await.unwrap();

        let dummy_block: Block<H256> = Block {
            number: ethers_core::types::U64::from(1).into(),
            hash: ethers_core::types::H256::random().into(),
            ..Default::default()
        };

        let dummy_receipt = new_storable_execution_result(
            ethers_core::types::H256::random().into(),
            dummy_block.hash.clone(),
            1_u64.into(),
        );

        db_client
            .insert_block_data(&[dummy_block.clone()], &[dummy_receipt.clone()], &[])
            .await
            .unwrap();

        let block = db_client.get_block_by_number(1).await.unwrap();

        assert_eq!(block.number.0.as_u64(), 1);
        assert_eq!(block.hash, dummy_block.hash);

        assert_eq!(block.transactions.len(), 0);

        let receipt = db_client
            .get_transaction_receipt(dummy_receipt.transaction_hash.clone())
            .await
            .unwrap();

        assert_eq!(receipt.transaction_hash, dummy_receipt.transaction_hash);
    })
    .await;
}

#[tokio::test]
async fn test_insertion_of_txs_and_receipts_with_no_blocks() {
    test_with_clients(|db_client| async move {
        db_client.init(None, false).await.unwrap();

        let dummy_txn = Transaction {
            hash: ethers_core::types::H256::random().into(),
            block_number: Some(1_u64.into()),
            ..Default::default()
        };

        let dummy_receipt = new_storable_execution_result(
            dummy_txn.hash.clone(),
            ethers_core::types::H256::random().into(),
            1_u64.into(),
        );

        db_client
            .insert_block_data(&[], &[dummy_receipt], &[dummy_txn.clone()])
            .await
            .unwrap();

        let receipt = db_client
            .get_transaction_receipt(dummy_txn.hash.clone())
            .await
            .unwrap();

        assert_eq!(receipt.transaction_hash, dummy_txn.hash);

        let block = db_client.get_block_by_number(1).await;

        assert!(block.is_err());
    })
    .await;
}

#[tokio::test]
async fn test_insert_and_fetch_genesis_accounts() {
    test_with_clients(|db_client| async move {
        // Arrange
        db_client.init(None, false).await.unwrap();

        let genesis_balances = vec![
            AccountBalance {
                address: H160::from(ethers_core::types::H160::random()),
                balance: U256::from(100_u64),
            },
            AccountBalance {
                address: H160::from(ethers_core::types::H160::random()),
                balance: U256::from(200_u64),
            },
        ];

        // There should be no genesis balances when the database is empty
        {
            // Act
            let balances = db_client.get_genesis_balances().await.unwrap();

            // Assert
            assert!(balances.is_none());
        }

        // Insert genesis balances
        {
            // Act
            db_client
                .insert_genesis_balances(&genesis_balances)
                .await
                .unwrap();

            let balances = db_client.get_genesis_balances().await.unwrap();

            // Assert
            assert!(balances.is_some());
            assert_eq!(balances.unwrap(), genesis_balances);
        }

        // There should be no genesis balances when the database is cleared
        {
            // Act
            db_client.clear().await.unwrap();
            let balances = db_client.get_genesis_balances().await.unwrap();

            // Assert
            assert!(balances.is_none());
        }
    })
    .await;
}

fn new_storable_execution_result(
    transaction_hash: H256,
    block_hash: H256,
    block_number: U64,
) -> StorableExecutionResult {
    StorableExecutionResult {
        transaction_hash,
        block_hash,
        exe_result: ExeResult::success(U256::max_value(), did::block::TransactOut::None, vec![]),
        transaction_index: Default::default(),
        block_number,
        from: Default::default(),
        to: Default::default(),
        transaction_type: Default::default(),
        cumulative_gas_used: Default::default(),
        max_fee_per_gas: Default::default(),
        gas_price: Default::default(),
        max_priority_fee_per_gas: Default::default(),
    }
}
