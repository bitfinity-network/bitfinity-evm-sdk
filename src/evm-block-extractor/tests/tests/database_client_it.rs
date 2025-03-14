use did::{Block, Transaction, H160, H256, U256, U64};
use evm_block_extractor::database::{AccountBalance, CertifiedBlock, DatabaseClient};
use rand::random;

use crate::test_with_clients;

#[tokio::test]
async fn test_batch_insertion_of_blocks_and_transactions_retrieval() {
    test_with_clients(|db_client| async move {
        db_client.init(None, false).await.unwrap();

        let mut blocks = Vec::new();

        for i in 1..=10 {
            let dummy_block: Block<H256> = Block {
                number: alloy::primitives::U64::from(i).into(),
                hash: alloy::primitives::B256::random().into(),
                ..Default::default()
            };

            blocks.push(dummy_block);
        }

        const TRANSACTIONS_PER_BLOCK: u64 = 10;

        let mut exe_results = Vec::new();

        for i in 1..=10 {
            for _ in 0..TRANSACTIONS_PER_BLOCK {
                let tx_hash = alloy::primitives::B256::random();
                exe_results.push(did::H256::from(tx_hash));
                blocks[i - 1].transactions.push(tx_hash.into());
            }
        }

        let mut txn = vec![];
        for i in 0..10 {
            for j in 0..TRANSACTIONS_PER_BLOCK {
                let tx_hash = &exe_results[(i * TRANSACTIONS_PER_BLOCK + j) as usize];
                let block_number = blocks[i as usize].number.0.to::<u64>();
                let dummy_txn = Transaction {
                    hash: tx_hash.clone(),
                    block_number: Some(U64::from(block_number)),
                    ..Default::default()
                };

                txn.push(dummy_txn);
            }
        }

        db_client.insert_block_data(&blocks, &txn).await.unwrap();

        let block = db_client.get_full_block_by_number(1).await.unwrap();

        // Check the transactions
        assert_eq!(block.transactions.len(), TRANSACTIONS_PER_BLOCK as usize);
        for i in 0..TRANSACTIONS_PER_BLOCK {
            assert_eq!(block.transactions[i as usize].hash, exe_results[i as usize]);
        }

        assert_eq!(block.number.0.to::<u64>(), 1);

        let tx = db_client
            .get_transaction(exe_results[0].clone())
            .await
            .unwrap();

        assert_eq!(tx.hash, exe_results[0].clone());

        let block = db_client.get_full_block_by_number(10).await.unwrap();

        assert_eq!(block.number.0.to::<u64>(), 10);

        let tx = db_client
            .get_transaction(exe_results[9 * TRANSACTIONS_PER_BLOCK as usize].clone())
            .await
            .unwrap();

        assert_eq!(tx.hash, exe_results[9 * TRANSACTIONS_PER_BLOCK as usize]);
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
                number: alloy::primitives::U64::from(i).into(),
                hash: alloy::primitives::B256::random().into(),
                ..Default::default()
            };

            db_client
                .insert_block_data(&[dummy_block], &[])
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
            number: alloy::primitives::U64::from(1).into(),
            hash: alloy::primitives::B256::random().into(),
            ..Default::default()
        };

        assert!(db_client
            .insert_block_data(&[dummy_block], &[])
            .await
            .is_err());

        // First initialization - creates tables
        db_client.init(None, false).await.unwrap();

        // Add a block
        let dummy_block: Block<H256> = Block {
            number: alloy::primitives::U64::from(1).into(),
            hash: alloy::primitives::B256::random().into(),
            ..Default::default()
        };

        assert!(db_client
            .insert_block_data(&[dummy_block], &[])
            .await
            .is_ok());

        assert!(db_client.init(None, false).await.is_ok());

        // Retrieve the block
        let block = db_client.get_block_by_number(1).await.unwrap();

        assert_eq!(block.number.0.to::<u64>(), 1);
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
                number: alloy::primitives::U64::from(i).into(),
                hash: alloy::primitives::B256::random().into(),
                ..Default::default()
            };

            blocks.push(dummy_block);
        }

        let mut txn = vec![];
        for _ in 0..10 {
            let dummy_txn = Transaction {
                hash: alloy::primitives::B256::random().into(),
                block_number: Some(5_u64.into()),
                block_hash: Some(blocks[4].hash.clone()),
                ..Default::default()
            };

            txn.push(dummy_txn);
        }

        blocks[4].transactions = txn.iter().map(|tx| tx.hash.clone()).collect();

        db_client.insert_block_data(&blocks, &txn).await.unwrap();

        let block = db_client.get_block_by_number(1).await.unwrap();

        // Check the transactions
        assert_eq!(block.transactions.len(), 0);

        assert_eq!(block.number.0.to::<u64>(), 1);

        let block = db_client.get_full_block_by_number(5).await.unwrap();

        assert_eq!(block.hash, blocks[4].hash);

        assert_eq!(block.number.0.to::<u64>(), 5);
        assert_eq!(block.transactions.len(), 10);

        for txn in block.transactions {
            assert_eq!(txn.block_number.unwrap().0.to::<u64>(), 5);
            assert_eq!(txn.block_hash.unwrap(), block.hash);
        }
    })
    .await;
}

#[tokio::test]
async fn test_deletion_and_creation_of_table_when_earliest_blocks_are_different() {
    test_with_clients(|db_client| async move {
        let block_one: Block<Transaction> = Block::<H256> {
            number: alloy::primitives::U64::from(0).into(),
            hash: alloy::primitives::B256::random().into(),
            ..Default::default()
        }
        .into_full_block(vec![])
        .unwrap();

        let block_two: Block<Transaction> = Block::<H256> {
            number: alloy::primitives::U64::from(0).into(),
            hash: alloy::primitives::B256::random().into(),
            ..Default::default()
        }
        .into_full_block(vec![])
        .unwrap();

        db_client.init(None, false).await.unwrap();

        assert!(db_client
            .insert_block_data(&[block_one.clone().into()], &[])
            .await
            .is_ok());

        let block = db_client.get_block_by_number(0).await.unwrap();

        assert_eq!(block.number.0.to::<u64>(), 0);
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
            .insert_block_data(&[block_two.clone().into()], &[])
            .await
            .is_ok());

        // Retrieve the block
        let block = db_client.get_block_by_number(0).await.unwrap();

        assert_eq!(block.number.0.to::<u64>(), 0);
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
                &[Block::<H256> {
                    number: U64::zero(),
                    hash: alloy::primitives::B256::random().into(),
                    ..Default::default()
                }],
                &[Transaction {
                    block_number: Some(U64::zero()),
                    hash: alloy::primitives::B256::random().into(),
                    ..Default::default()
                }],
            )
            .await
            .unwrap();

        let block = db_client.get_block_by_number(0).await.unwrap();
        assert_eq!(block.number.0.to::<u64>(), 0);

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
            number: alloy::primitives::U64::from(1).into(),
            hash: alloy::primitives::B256::random().into(),
            ..Default::default()
        };

        db_client
            .insert_block_data(&[dummy_block.clone()], &[])
            .await
            .unwrap();

        let same_block = db_client
            .check_if_same_block_hash(&dummy_block)
            .await
            .unwrap();

        assert!(same_block);

        let dummy_block: Block<H256> = Block {
            number: alloy::primitives::U64::from(1).into(),
            hash: alloy::primitives::B256::random().into(),
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
async fn test_insertion_of_blocks_with_txs() {
    test_with_clients(|db_client| async move {
        db_client.init(None, false).await.unwrap();

        let dummy_txn = Transaction {
            hash: alloy::primitives::B256::random().into(),
            block_number: Some(1_u64.into()),
            ..Default::default()
        };

        let dummy_block: Block<H256> = Block {
            number: alloy::primitives::U64::from(1).into(),
            hash: alloy::primitives::B256::random().into(),
            transactions: vec![dummy_txn.hash.clone()],
            ..Default::default()
        };

        db_client
            .insert_block_data(&[dummy_block.clone()], &[dummy_txn.clone()])
            .await
            .unwrap();

        let block = db_client.get_full_block_by_number(1).await.unwrap();

        assert_eq!(block.number.0.to::<u64>(), 1);
        assert_eq!(block.hash, dummy_block.hash);

        assert_eq!(block.transactions.len(), 1);
        assert_eq!(block.transactions[0].hash, dummy_txn.hash);
    })
    .await;
}

#[tokio::test]
async fn test_insertion_of_blocks_with_no_txs() {
    test_with_clients(|db_client| async move {
        db_client.init(None, false).await.unwrap();

        let dummy_block: Block<H256> = Block {
            number: alloy::primitives::U64::from(1).into(),
            hash: alloy::primitives::B256::random().into(),
            ..Default::default()
        };

        db_client
            .insert_block_data(&[dummy_block.clone()], &[])
            .await
            .unwrap();

        let block = db_client.get_block_by_number(1).await.unwrap();

        assert_eq!(block.number.0.to::<u64>(), 1);
        assert_eq!(block.hash, dummy_block.hash);

        assert_eq!(block.transactions.len(), 0);
    })
    .await;
}

#[tokio::test]
async fn test_insertion_of_txs_with_no_blocks() {
    test_with_clients(|db_client| async move {
        db_client.init(None, false).await.unwrap();

        let dummy_txn = Transaction {
            hash: alloy::primitives::B256::random().into(),
            block_number: Some(1_u64.into()),
            ..Default::default()
        };

        db_client
            .insert_block_data(&[], &[dummy_txn.clone()])
            .await
            .unwrap();

        let tx = db_client
            .get_transaction(dummy_txn.hash.clone())
            .await
            .unwrap();

        assert_eq!(tx.hash, dummy_txn.hash);

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
                address: H160::from(alloy::primitives::Address::random()),
                balance: U256::from(100_u64),
            },
            AccountBalance {
                address: H160::from(alloy::primitives::Address::random()),
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

#[tokio::test]
async fn test_insert_and_fetch_chain_id() {
    test_with_clients(|db_client| async move {
        // Arrange
        db_client.init(None, false).await.unwrap();

        let chain_id: u64 = random();

        // There should be no chain when the database is empty
        {
            // Act
            let chain_id_from_db = db_client.get_chain_id().await.unwrap();

            // Assert
            assert!(chain_id_from_db.is_none());
        }

        // Insert chain id
        {
            // Act
            db_client.insert_chain_id(chain_id).await.unwrap();

            let chain_id_from_db = db_client.get_chain_id().await.unwrap();

            // Assert
            assert!(chain_id_from_db.is_some());
            assert_eq!(chain_id_from_db.unwrap(), chain_id);
        }

        // There should be no chain id when the database is cleared
        {
            // Act
            db_client.clear().await.unwrap();
            let chain_id_from_db = db_client.get_chain_id().await.unwrap();

            // Assert
            assert!(chain_id_from_db.is_none());
        }
    })
    .await;
}

#[tokio::test]
async fn test_insert_and_fetch_last_block_certified_data() {
    test_with_clients(|db_client| async move {
        // Arrange
        db_client.init(None, false).await.unwrap();

        let block_1 = Block::<H256> {
            number: 1u64.into(),
            ..Default::default()
        };
        let block_2 = Block::<H256> {
            number: 2u64.into(),
            ..Default::default()
        };

        let result_1 = CertifiedBlock {
            certificate: vec![1, 2, 3],
            witness: vec![5, 6, 7],
            data: block_1,
        };
        let result_2 = CertifiedBlock {
            certificate: vec![11, 12, 13],
            witness: vec![15, 16, 17],
            data: block_2,
        };

        // There should be no certified block when the database is empty
        {
            // Act
            let certified_data = db_client.get_last_certified_block_data().await;

            // Assert
            assert!(certified_data.is_err());
        }

        // Insert certified blocks
        {
            db_client
                .insert_certified_block_data(result_1.clone())
                .await
                .unwrap();
            assert_eq!(
                result_1,
                db_client.get_last_certified_block_data().await.unwrap()
            );

            db_client
                .insert_certified_block_data(result_2.clone())
                .await
                .unwrap();
            assert_eq!(
                result_2,
                db_client.get_last_certified_block_data().await.unwrap()
            );
        }

        // There should be no certified blocks when the database is cleared
        {
            // Act
            db_client.clear().await.unwrap();
            let certified_data = db_client.get_last_certified_block_data().await;

            // Assert
            assert!(certified_data.is_err());
        }
    })
    .await;
}

#[tokio::test]
async fn test_blockchain_tail_discard_and_get_discarded_entries() {
    test_with_clients(|db_client| async move {
        db_client.init(None, false).await.unwrap();

        let mut blocks = Vec::new();

        const FIRST_BLOCK: u64 = 1;
        const LAST_BLOCK: u64 = 10;

        for i in FIRST_BLOCK..=LAST_BLOCK {
            let dummy_block: Block<H256> = Block {
                number: alloy::primitives::U64::from(i).into(),
                hash: alloy::primitives::B256::random().into(),
                ..Default::default()
            };

            blocks.push(dummy_block);
        }

        const TRANSACTIONS_PER_BLOCK: u64 = 10;

        for i in FIRST_BLOCK..=LAST_BLOCK {
            for _ in 0..TRANSACTIONS_PER_BLOCK {
                let tx_hash = alloy::primitives::B256::random();
                blocks[i as usize - 1].transactions.push(tx_hash.into());
            }
        }

        let mut txn = vec![];
        for i in 0..10 {
            for j in 0..TRANSACTIONS_PER_BLOCK {
                let tx_hash = blocks[i as usize].transactions[j as usize].clone();
                let block_number = blocks[i as usize].number.0.to::<u64>();
                let dummy_txn = Transaction {
                    hash: tx_hash,
                    block_number: Some(U64::from(block_number)),
                    ..Default::default()
                };

                txn.push(dummy_txn);
            }
        }

        db_client.insert_block_data(&blocks, &txn).await.unwrap();

        let certified_block = CertifiedBlock {
            certificate: vec![1, 2, 3],
            witness: vec![5, 6, 7],
            data: blocks.last().unwrap().clone(),
        };
        db_client
            .insert_certified_block_data(certified_block)
            .await
            .unwrap();

        // Discard two latest blocks with it's transactions.
        // Certified block should be alse removed.
        const FIRST_REMOVED_BLOCK: u64 = LAST_BLOCK - 1;
        db_client
            .discard_blocks_from(FIRST_REMOVED_BLOCK, "test reason")
            .await
            .unwrap();

        assert!(
            check_blocks_with_txs_storage_state(
                &*db_client,
                &blocks[..FIRST_REMOVED_BLOCK as usize - 1],
                StorageState::Present,
            )
            .await
        );

        assert!(
            check_blocks_with_txs_storage_state(
                &*db_client,
                &blocks[FIRST_REMOVED_BLOCK as usize - 1..],
                StorageState::NotPresent,
            )
            .await
        );

        let certified_block_result = db_client.get_last_certified_block_data().await;
        assert!(certified_block_result.is_err());

        // Check if blocks and txs present in DB as discarded.
        for block in &blocks[FIRST_REMOVED_BLOCK as usize..] {
            let discarded = db_client
                .get_discarded_block_by_hash(block.hash.clone())
                .await
                .unwrap();

            assert!(discarded
                .block
                .transactions
                .iter()
                .zip(block.transactions.iter())
                .all(|(a, b)| &a.hash == b));
        }
    })
    .await;
}

async fn check_blocks_with_txs_storage_state(
    db_client: &dyn DatabaseClient,
    blocks: &[Block<did::H256>],
    storage_state: StorageState,
) -> bool {
    for block in blocks {
        let block_result = db_client.get_block_by_number(block.number.as_u64()).await;
        if !storage_state.check(&block_result) {
            return false;
        }

        for tx in &block.transactions {
            let tx_result = db_client.get_transaction(tx.clone()).await;
            if !storage_state.check(&tx_result) {
                return false;
            }
        }
    }

    true
}

#[derive(Debug)]
enum StorageState {
    Present,
    NotPresent,
}

impl StorageState {
    pub fn check<T, E>(&self, query_result: &Result<T, E>) -> bool {
        match self {
            StorageState::Present => query_result.is_ok(),
            StorageState::NotPresent => query_result.is_err(),
        }
    }
}
