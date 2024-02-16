pub mod big_query_db_client;
pub mod postgres_db_client;

use did::{Block, Transaction, H160, H256, U256};
use serde::{Deserialize, Serialize};

/// Account balance
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccountBalance {
    pub address: H160,
    pub balance: U256,
}

/// Generic data container
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DataContainer<D> {
    pub data: D,
}

impl<D> DataContainer<D> {
    pub fn new(data: D) -> Self {
        Self { data }
    }
}

/// The genesis balances key in the key value store
const GENESIS_BALANCES_KEY: &str = "genesis_balances";
/// The chain id key in the key value store
const CHAIN_ID_KEY: &str = "chain_id";

/// A trait for interacting with a blockchain database
#[async_trait::async_trait]
pub trait DatabaseClient: Send + Sync {
    /// Initialize the database
    async fn init(&self, block: Option<Block<H256>>, reset_database: bool) -> anyhow::Result<()>;

    /// Delete/clear the tables
    async fn clear(&self) -> anyhow::Result<()>;

    /// Returns whether the block hash corresponds to the one in the db
    async fn check_if_same_block_hash(&self, block: &Block<H256>) -> anyhow::Result<bool> {
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

    /// Insert block data; this includes transactions and the blocks
    async fn insert_block_data(
        &self,
        blocks: &[Block<H256>],
        transactions: &[Transaction],
    ) -> anyhow::Result<()>;

    /// Get genesis balances
    async fn get_genesis_balances(&self) -> anyhow::Result<Option<Vec<AccountBalance>>>;

    /// Insert genesis balances
    async fn insert_genesis_balances(
        &self,
        genesis_balances: &[AccountBalance],
    ) -> anyhow::Result<()>;

    /// Get chain id
    async fn get_chain_id(&self) -> anyhow::Result<Option<u64>>;

    /// Insert chain_id
    async fn insert_chain_id(&self, chain_id: u64) -> anyhow::Result<()>;

    /// Get a transaction from the database
    async fn get_transaction(&self, tx_hash: H256) -> anyhow::Result<Transaction>;

    /// Get the latest block number
    async fn get_latest_block_number(&self) -> anyhow::Result<Option<u64>>;

    /// Get earliest block number
    async fn get_earliest_block_number(&self) -> anyhow::Result<u64>;
}
