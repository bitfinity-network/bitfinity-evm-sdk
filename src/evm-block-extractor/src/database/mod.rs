pub mod postgres_db_client;

use chrono::{DateTime, Utc};
use did::certified::CertifiedResult;
use did::{Block, BlockchainBlockInfo, H160, H256, Transaction, U256};
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
pub trait DatabaseClient: Send + Sync {
    /// Initialize the database
    fn init(
        &self,
        block: Option<Block<H256>>,
        reset_database: bool,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;

    /// Delete/clear the tables
    fn clear(&self) -> impl Future<Output = anyhow::Result<()>> + Send;

    /// Returns whether the block hash corresponds to the one in the db
    fn check_if_same_block_hash(
        &self,
        block: &Block<H256>,
    ) -> impl Future<Output = anyhow::Result<bool>> + Send {
        async {
            let block_number = block.number.0.to();
            let block_in_db = self.get_block_by_number(block_number).await?;
            Ok(block.hash == block_in_db.hash)
        }
    }

    /// Get a block from the database
    fn get_block_by_number(
        &self,
        block_number: u64,
    ) -> impl Future<Output = anyhow::Result<Block<H256>>> + Send;

    /// Get a block from the database
    fn get_full_block_by_number(
        &self,
        block_number: u64,
    ) -> impl Future<Output = anyhow::Result<Block<Transaction>>> + Send;

    /// Insert block data; this includes transactions and the blocks
    fn insert_block_data(
        &self,
        blocks: &[Block<H256>],
        transactions: &[Transaction],
    ) -> impl Future<Output = anyhow::Result<()>> + Send;

    /// Insert certified block data
    fn insert_certified_block_data(
        &self,
        response: CertifiedBlock,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;

    /// Returns certified response for the last block
    fn get_last_certified_block_data(
        &self,
    ) -> impl Future<Output = anyhow::Result<CertifiedBlock>> + Send;

    /// Get genesis balances
    fn get_genesis_balances(
        &self,
    ) -> impl Future<Output = anyhow::Result<Option<Vec<AccountBalance>>>> + Send;

    /// Insert genesis balances
    fn insert_genesis_balances(
        &self,
        genesis_balances: &[AccountBalance],
    ) -> impl Future<Output = anyhow::Result<()>> + Send;

    /// Get chain id
    fn get_chain_id(&self) -> impl Future<Output = anyhow::Result<Option<u64>>> + Send;

    /// Insert chain_id
    fn insert_chain_id(&self, chain_id: u64) -> impl Future<Output = anyhow::Result<()>> + Send;

    /// Get a transaction from the database
    fn get_transaction(
        &self,
        tx_hash: H256,
    ) -> impl Future<Output = anyhow::Result<Transaction>> + Send;

    /// Get the latest block number
    fn get_latest_block_number(&self) -> impl Future<Output = anyhow::Result<Option<u64>>> + Send;

    /// Get earliest block number
    fn get_earliest_block_number(&self) -> impl Future<Output = anyhow::Result<u64>> + Send;

    /// Delete latest blocks starting with `start_from`, and related transactions.
    /// Deleted blocks and transactions will be preserved in 'discarded' table with
    /// the given 'reason' and timestamp.
    fn discard_blocks_from(
        &self,
        start_from: u64,
        reason: &str,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;

    /// Returns a discarded block by its hash.
    fn get_discarded_block_by_hash(
        &self,
        block_hash: H256,
    ) -> impl Future<Output = anyhow::Result<DiscardedBlock>> + Send;

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
    fn get_block_info(
        &self,
    ) -> impl Future<Output = anyhow::Result<Option<BlockchainBlockInfo>>> + Send;

    /// Stores blockchain block info.
    fn set_block_info(
        &self,
        info: BlockchainBlockInfo,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;
}

/// Discarded block with metadata.
#[derive(Debug)]
pub struct DiscardedBlock {
    pub block: Block<Transaction>,
    pub reason: String,
    pub timestamp: DateTime<Utc>,
}
