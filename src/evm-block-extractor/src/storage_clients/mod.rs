
pub mod gcp_big_query;
pub mod hashmap;

use ethers_core::types::{Block, Transaction};

#[async_trait::async_trait]
pub trait BlockChainDB: Send + Sync  {
    async fn get_block_by_number(&self, block: u64) -> anyhow::Result<Block<Transaction>>;
    async fn insert_block(&mut self, block: Block<Transaction>) -> anyhow::Result<()>;
    async fn get_blocks_in_range(&self, start: u64, end: u64) -> anyhow::Result<Vec<u64>>;

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
}


pub enum StorageClient {
    HashMap,
    BigQuery{agent_key_path: String},
}


