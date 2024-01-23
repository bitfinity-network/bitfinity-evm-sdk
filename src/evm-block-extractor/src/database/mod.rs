pub mod big_query_db_client;
pub mod postgres_db_client;

use did::transaction::StorableExecutionResult;
use did::{Block, Transaction, TransactionReceipt, H256};

/// A trait for interacting with a blockchain database
#[async_trait::async_trait]
pub trait DatabaseClient: Send + Sync {
    /// Initialize the database
    async fn init(&self, block: Option<Block<H256>>, reset_database: bool) -> anyhow::Result<()>;

    /// Delete/clear the tables
    async fn clear(&self) -> anyhow::Result<()>;

    /// Returns whether the block hash corresponds to the one in the db
    async fn check_if_same_block_hash(&self, block: Block<H256>) -> anyhow::Result<bool> {
        let block_number = block.number.0.as_u64();
        let block_in_db = self.get_block_by_number(block_number).await?;
        Ok(block.hash == block_in_db.hash)
    }

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
