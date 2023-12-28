pub mod gcp_bq;
pub mod hashmap;

use ethers_core::types::{Block, Transaction, TransactionReceipt};

/// A trait for interacting with a blockchain database
#[async_trait::async_trait]
pub trait BlockChainDB: Send + Sync {
    /// Get a block from the database
    async fn get_block_by_number(&self, block: u64) -> anyhow::Result<Block<Transaction>>;

    /// Insert a block into the database
    async fn insert_block(&mut self, block: &Block<Transaction>) -> anyhow::Result<()>;

    async fn get_blocks_in_range(&self, start: u64, end: u64) -> anyhow::Result<Vec<u64>>;

    /// Get the missing blocks in a range
    async fn get_missing_blocks_in_range(&self, start: u64, end: u64) -> anyhow::Result<Vec<u64>> {
        let all_blocks = self.get_blocks_in_range(start, end).await?;
        let mut missing_blocks = Vec::new();
        for block_num in start..=end {
            if !all_blocks.contains(&block_num) {
                missing_blocks.push(block_num);
            }
        }
        Ok(missing_blocks)
    }

    /// Insert receipts into the database
    async fn insert_receipts(&mut self, receipts: TransactionReceipt) -> anyhow::Result<()>;

    /// Get a transaction receipt from the database
    async fn get_transaction_receipt(&self, tx_hash: String) -> anyhow::Result<TransactionReceipt>;
}
