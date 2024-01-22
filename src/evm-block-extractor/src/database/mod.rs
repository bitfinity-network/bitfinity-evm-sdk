pub mod big_query_db_client;
pub mod postgres_db_client;

use did::transaction::StorableExecutionResult;
use did::{Block, Transaction, TransactionReceipt, H256};

use crate::constants::{MAINNET_PREFIX, TESTNET_PREFIX};

/// A trait for interacting with a blockchain database
#[async_trait::async_trait]
pub trait DatabaseClient: Send + Sync {
    /// Initialize the database
    async fn init(&self, block: Option<Block<H256>>, reset_database: bool) -> anyhow::Result<()>;

    /// Delete/clear the tables
    async fn clear(&self) -> anyhow::Result<()>;

    /// Get a block from the database
    async fn get_block_by_number(&self, block_number: u64) -> anyhow::Result<Block<H256>>;

    /// Get a block from the database
    async fn get_full_block_by_number(
        &self,
        block_number: u64,
    ) -> anyhow::Result<Block<Transaction>>;

    /// Insert block data; these include receipts, transactions and the blocks
    async fn insert_block_data(
        &self,
        blocks: &[Block<H256>],
        receipts: &[StorableExecutionResult],
        transactions: &[Transaction],
    ) -> anyhow::Result<()>;

    /// Get a transaction receipt from the database
    async fn get_transaction_receipt(&self, tx_hash: H256) -> anyhow::Result<TransactionReceipt>;

    /// Get the latest block number
    async fn get_latest_block_number(&self) -> anyhow::Result<Option<u64>>;

    /// Get earliest block number
    async fn get_earliest_block_number(&self) -> anyhow::Result<u64>;
}

/// Reset the database if needed
async fn reset_database_if_needed(
    db: &dyn DatabaseClient,
    database_type: &str,
    block: Option<Block<H256>>,
    mut reset_database: bool,
) -> anyhow::Result<()> {
    if database_type.contains(TESTNET_PREFIX) {
        reset_database = true;
    }

    if reset_database {
        if database_type.contains(MAINNET_PREFIX) {
            panic!("Cannot reset the mainnet database");
        }
        let Some(block) = block else {
            panic!("Cannot reset the database without earliest block");
        };

        let block_number = block.number.expect("Block number not found").as_u64();

        let block_in_db = db
            .get_block_by_number(block_number)
            .await
            .expect("Block not found in the database, The Database cannot be rebuilt");

        let block_hash = block_in_db.hash.expect("Block hash not found");

        if block.hash.expect("should be present") != block_hash && !block_hash.is_zero() {
            db.clear().await?;
        }
    }

    Ok(())
}
