
pub mod gcp_big_query;
pub mod hashmap;

use ethers_core::types::{Block, Transaction};

#[async_trait::async_trait]
pub trait BlockChainDB: Send + Sync  {
    async fn get_block_by_number(&self, block: u64) -> anyhow::Result<Block<Transaction>>;
    async fn insert_block(&mut self, block: Block<Transaction>) -> anyhow::Result<()>;
    async fn get_last_block_number(&self) -> anyhow::Result<u64>;
}


pub enum StorageClient {
    HashMap,
    BigQuery{agent_key_path: String},
}


