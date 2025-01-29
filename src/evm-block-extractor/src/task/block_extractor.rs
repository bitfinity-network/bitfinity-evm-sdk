use std::sync::Arc;

use did::evm_state::EvmGlobalState;
use did::BlockNumber;
use ethereum_json_rpc_client::{Client, EthJsonRpcClient};
use log::*;
use tokio::time::Duration;

use crate::config::ExtractorArgs;
use crate::database::{AccountBalance, CertifiedBlock, DatabaseClient};

/// Starts the block extractor process
pub async fn start_extractor<C: Client>(
    config: ExtractorArgs,
    db_client: Arc<dyn DatabaseClient>,
    evm_client: Arc<EthJsonRpcClient<C>>,
) -> anyhow::Result<()> {
    let earliest_block = evm_client
        .get_block_by_number(BlockNumber::Earliest)
        .await?;

    db_client
        .init(Some(earliest_block), config.reset_db_on_state_change)
        .await?;

    let mut extractor = BlockExtractor::new(
        evm_client.clone(),
        config.request_time_out_secs,
        config.rpc_batch_size,
        db_client.clone(),
    );

    let end_block = evm_client.get_block_number().await?;
    debug!("latest block number in evm: {}", end_block);

    let start_block = db_client.get_latest_block_number().await?;
    debug!("latest block number stored: {:?}", start_block);

    extractor
        .collect_all(start_block.map(|b| b + 1).unwrap_or_default(), end_block)
        .await?;

    Ok(())
}

/// Extracts blocks from an EVMC and stores them in a database
pub struct BlockExtractor<C: Client> {
    client: Arc<EthJsonRpcClient<C>>,
    request_time_out_secs: u64,
    rpc_batch_size: usize,
    blockchain: Arc<dyn DatabaseClient>,
}

/// Outcome of the block extraction process
pub enum BlockExtractCollectOutcome {
    /// No blocks were extracted because EVM global state is not enabled
    BlocksNotExtracted,
    /// Blocks were extracted
    BlocksExtracted { from_block: u64, to_block: u64 },
}

impl<C: Client> BlockExtractor<C> {
    pub fn new(
        client: Arc<EthJsonRpcClient<C>>,
        request_time_out_secs: u64,
        rpc_batch_size: usize,
        blockchain: Arc<dyn DatabaseClient>,
    ) -> Self {
        Self {
            client,
            blockchain,
            rpc_batch_size,
            request_time_out_secs,
        }
    }

    /// Collects blocks from the EVMC and stores them in the database.
    /// Returns the inclusive range of blocks that were collected.
    /// This collects also the genesis accounts if needed.
    pub async fn collect_all(
        &mut self,
        from_block_inclusive: u64,
        to_block_inclusive: u64,
    ) -> anyhow::Result<BlockExtractCollectOutcome> {
        match self.client.get_evm_global_state().await {
            Ok(EvmGlobalState::Enabled) => {
                debug!("EVM global state is enabled.");
            }
            Ok(state) => {
                warn!(
                    "EVM global state is not enabled: {:?}. Blocks will not be extracted.",
                    state
                );
                return Ok(BlockExtractCollectOutcome::BlocksNotExtracted);
            }
            // We can't get the EVM global state if the evm-canister version is too old.
            // Once all the canisters are updated, we can remove this logic and return instead of proceed.
            // TODO: Remove this logic in EPROD-1123
            Err(e) => {
                warn!(
                    "Error getting EVM global state: {:?}. The blocks will be extracted anyway.",
                    e
                );
            }
        }

        self.collect_chain_id().await?;
        self.collect_genesis_balances().await?;
        self.collect_last_certified_block().await?;

        info!(
            "Getting blocks from {:?} to {}",
            from_block_inclusive, to_block_inclusive
        );
        let client = self.client.clone();

        let request_time_out_secs = self.request_time_out_secs;
        let batch_size = self.rpc_batch_size;

        let mut next_from = from_block_inclusive;

        while next_from <= to_block_inclusive {
            let to = (to_block_inclusive + 1).min(next_from + batch_size as u64);
            let blocks_batch = next_from..to;
            next_from = to;

            let block_numbers = blocks_batch
                .into_iter()
                .map(|block| BlockNumber::Number(block.into()));

            let evm_blocks = tokio::time::timeout(
                Duration::from_secs(request_time_out_secs),
                client.get_full_blocks_by_number(block_numbers, batch_size),
            )
            .await??;

            let all_transactions = evm_blocks
                .iter()
                .flat_map(|block| &block.transactions)
                .cloned()
                .collect::<Vec<_>>();

            let blocks = evm_blocks
                .into_iter()
                .map(|block| block.into())
                .collect::<Vec<did::Block<did::H256>>>();

            let all_transactions = all_transactions
                .into_iter()
                .collect::<Vec<did::Transaction>>();

            self.blockchain
                .insert_block_data(&blocks, &all_transactions)
                .await?;
        }

        Ok(BlockExtractCollectOutcome::BlocksExtracted {
            from_block: from_block_inclusive,
            to_block: to_block_inclusive,
        })
    }

    /// Collects last certified block
    async fn collect_last_certified_block(&self) -> anyhow::Result<()> {
        let certified_block = self.client.get_last_certified_block().await?;
        self.blockchain
            .insert_certified_block_data(CertifiedBlock {
                data: certified_block.data,
                witness: certified_block.witness,
                certificate: certified_block.certificate,
            })
            .await?;

        Ok(())
    }

    /// Collects the genesis accounts if needed.
    async fn collect_genesis_balances(&self) -> anyhow::Result<()> {
        if self.blockchain.get_genesis_balances().await?.is_some() {
            debug!("Genesis balances already present in the DB. Skipping");
            return Ok(());
        }

        info!("Genesis balances not present in the DB. Collecting them");

        match self.client.get_genesis_balances().await {
            Ok(genesis_balances) => {
                let genesis_balances = genesis_balances
                    .into_iter()
                    .map(|(address, balance)| AccountBalance { address, balance })
                    .collect::<Vec<_>>();
                self.blockchain
                    .insert_genesis_balances(&genesis_balances)
                    .await?;
            }
            Err(e) => {
                error!("Error getting genesis balances: {:?}. The process will not be stopped but there will be missing genesis balances in the DB", e);
            }
        }

        Ok(())
    }

    /// Collects the chain_id if needed.
    async fn collect_chain_id(&self) -> anyhow::Result<()> {
        if self.blockchain.get_chain_id().await?.is_some() {
            debug!("Chain id already present in the DB. Skipping");
            return Ok(());
        }

        info!("Chain id not present in the DB. Collecting it");

        match self.client.get_chain_id().await {
            Ok(chain_id) => {
                self.blockchain.insert_chain_id(chain_id).await?;
            }
            Err(e) => {
                error!("Error getting chain id: {:?}. The process will not be stopped but the chain id will be missing in the DB", e);
            }
        }

        Ok(())
    }
}
