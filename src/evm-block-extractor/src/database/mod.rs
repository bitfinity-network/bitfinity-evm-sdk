pub mod big_query_db_client;
pub mod in_memory_db_client;
pub mod postgres_db_client;

use ethers_core::types::{Block, Transaction, TransactionReceipt, H256};

/// A trait for interacting with a blockchain database
#[async_trait::async_trait]
pub trait DatabaseClient: Send + Sync {
    /// Initialize the database
    async fn init(&self, block: Option<Block<H256>>) -> anyhow::Result<()>;

    /// Get a block from the database
    async fn get_block_by_number(&self, block: u64) -> anyhow::Result<Block<Transaction>>;

    /// Insert blocks and receipts into the database
    async fn insert_blocks_and_receipts(
        &self,
        blocks: &[Block<Transaction>],
        receipts: &[TransactionReceipt],
    ) -> anyhow::Result<()>;

    /// Get a transaction receipt from the database
    async fn get_transaction_receipt(&self, tx_hash: H256) -> anyhow::Result<TransactionReceipt>;

    /// Get the latest block number
    async fn get_latest_block_number(&self) -> anyhow::Result<Option<u64>>;

    /// Get earliest block number
    async fn get_earliest_block_number(&self) -> anyhow::Result<u64>;
}
