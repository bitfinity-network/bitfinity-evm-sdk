pub mod postgres_db_client;

use chrono::{DateTime, Utc};
use did::certified::CertifiedResult;
use did::{Block, BlockchainBlockInfo, Transaction, H160, H256, U256};
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
/// The blockchain block info key in the key value store
const BLOCKCHAIN_BLOCK_INFO_KEY: &str = "blockchain_block_info";

/// Certified block data
pub type CertifiedBlock = CertifiedResult<Block<H256>>;

/// A trait for interacting with a blockchain database
#[async_trait::async_trait]
pub trait DatabaseClient: Send + Sync {
    /// Initialize the database
    async fn init(&self, block: Option<Block<H256>>, reset_database: bool) -> anyhow::Result<()>;

    /// Delete/clear the tables
    async fn clear(&self) -> anyhow::Result<()>;

    /// Returns whether the block hash corresponds to the one in the db
    async fn check_if_same_block_hash(&self, block: &Block<H256>) -> anyhow::Result<bool> {
        let block_number = block.number.0.to();
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

    /// Insert certified block data
    async fn insert_certified_block_data(&self, response: CertifiedBlock) -> anyhow::Result<()>;

    /// Returns certified response for the last block
    async fn get_last_certified_block_data(&self) -> anyhow::Result<CertifiedBlock>;

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

    /// Delete latest blocks starting with `start_from`, and related transactions.
    /// Deleted blocks and transactions will be preserved in 'discarded' table with
    /// the given 'reason' and timestamp.
    async fn discard_blocks_from(&self, start_from: u64, reason: &str) -> anyhow::Result<()>;

    /// Returns a discarded block by its hash.
    async fn get_discarded_block_by_hash(&self, block_hash: H256)
        -> anyhow::Result<DiscardedBlock>;

    /// Returns block info from storage.
    ///
    /// # Warning
    /// Do not use this info fields as indexes for blocks in storage.
    /// The following numbers are about block numbers in the source blockchain
    /// and can exceed the latest block number in the database.
    /// - latest_block_number: u64,
    /// - safe_block_number: u64,
    /// - finalized_block_number: u64,
    /// - pending_block_number: u64,
    async fn get_block_info(&self) -> anyhow::Result<Option<BlockchainBlockInfo>>;

    /// Stores blockchain block info.
    async fn set_block_info(&self, info: BlockchainBlockInfo) -> anyhow::Result<()>;
}

/// Discarded block with metadata.
#[derive(Debug)]
pub struct DiscardedBlock {
    pub block: Block<Transaction>,
    pub reason: String,
    pub timestamp: DateTime<Utc>,
}
