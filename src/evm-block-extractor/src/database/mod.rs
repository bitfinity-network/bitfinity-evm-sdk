pub mod big_query_db_client;
pub mod in_memory_db_client;
pub mod postgres_db_client;

use ethers_core::types::{Block, Transaction, TransactionReceipt, H256};

/// A trait for interacting with a blockchain database
#[async_trait::async_trait]
pub trait DatabaseClient: Send + Sync {
    /// Initialize the database
    async fn init(&self) -> anyhow::Result<()>;

    /// Get a block from the database
    async fn get_block_by_number(
        &self,
        block: u64,
        include_transactions: bool,
    ) -> anyhow::Result<serde_json::Value>;

    /// Insert block data; these include receipts, transactions and the blocks
    async fn insert_block_data(
        &self,
        blocks: &[Block<H256>],
        receipts: &[TransactionReceipt],
        transactions: &[Transaction],
    ) -> anyhow::Result<()>;

    /// Get a transaction receipt from the database
    async fn get_transaction_receipt(&self, tx_hash: H256) -> anyhow::Result<TransactionReceipt>;

    /// Get the latest block number
    async fn get_latest_block_number(&self) -> anyhow::Result<Option<u64>>;

    /// Get earliest block number
    async fn get_earliest_block_number(&self) -> anyhow::Result<u64>;
}
