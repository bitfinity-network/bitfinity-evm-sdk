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
#[derive(Debug)]
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

        // Can't set the block info to the DB right now, because we can't be sure
        // the new block_info.safe_block numbers match with blocks in DB.
        // This must be set to DB after successful validation of the new blocks sequence.
        let block_info = self.client.get_blockchain_block_info().await?;

        info!(
            "Getting blocks from {:?} to {}",
            from_block_inclusive, to_block_inclusive
        );

        let mut next_from = from_block_inclusive;

        while next_from <= to_block_inclusive {
            let evm_blocks = self.fetch_new_blocks(next_from, to_block_inclusive).await?;

            if let Some(last_new_block) = evm_blocks.last() {
                next_from = last_new_block.number.as_u64() + 1;
            }

            self.validate(&evm_blocks).await?;

            self.persist_data(evm_blocks).await?;
        }

        // Now we are sure the numbers in the `block_info` describe
        // correct block sequence in DB.
        self.blockchain.set_block_info(block_info).await?;

        Ok(BlockExtractCollectOutcome::BlocksExtracted {
            from_block: from_block_inclusive,
            to_block: to_block_inclusive,
        })
    }

    /// Fetch new blocks from the EVM client.
    async fn fetch_new_blocks(
        &self,
        from: u64,
        to_block_inclusive: u64,
    ) -> Result<Vec<did::Block<did::Transaction>>, anyhow::Error> {
        let request_time_out_secs = self.request_time_out_secs;
        let batch_size = self.rpc_batch_size;

        let to = (to_block_inclusive + 1).min(from + batch_size as u64);
        let blocks_batch = from..to;
        let block_numbers = blocks_batch
            .into_iter()
            .map(|block| BlockNumber::Number(block.into()));
        let evm_blocks = tokio::time::timeout(
            Duration::from_secs(request_time_out_secs),
            self.client
                .get_full_blocks_by_number(block_numbers, batch_size),
        )
        .await??;
        Ok(evm_blocks)
    }

    /// Validate chain consistency, including new blocks sequence.
    async fn validate(
        &mut self,
        evm_blocks: &[did::Block<did::Transaction>],
    ) -> Result<(), anyhow::Error> {
        let latest_storage_block_number = self.blockchain.get_latest_block_number().await?;
        let latest_block = match latest_storage_block_number {
            Some(n) => self.blockchain.get_block_by_number(n).await.ok(),
            None => None,
        };

        let validation_result = Self::validate_chain(latest_block, evm_blocks);
        if let Err(e) = validation_result {
            self.process_validation_error(&e).await?;
            return Err(e.into());
        }

        Ok(())
    }

    /// Store the given blocks in database.
    async fn persist_data(
        &mut self,
        evm_blocks: Vec<did::Block<did::Transaction>>,
    ) -> Result<(), anyhow::Error> {
        let all_transactions = evm_blocks
            .iter()
            .flat_map(|block| &block.transactions)
            .cloned()
            .collect::<Vec<_>>();

        let blocks = evm_blocks
            .into_iter()
            .map(|block| block.into())
            .collect::<Vec<did::Block<did::H256>>>();

        self.blockchain
            .insert_block_data(&blocks, &all_transactions)
            .await?;

        Ok(())
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

    /// This function:
    /// - checks if the `new_blocks` sequence have correct hashes.
    /// - checks if `latest_block_in_storage.hash == new_blocks[0].prev_block_hash`.
    fn validate_chain<T1, T2>(
        latest_block_in_storage: Option<did::Block<T1>>,
        new_blocks: &[did::Block<T2>],
    ) -> Result<(), ChainError> {
        // if there are no blocks in storage, we don't need parent hash of
        // first new block
        let to_skip = if latest_block_in_storage.is_none() {
            1
        } else {
            0
        };
        let new_blocks_parent_hashes = new_blocks.iter().map(|b| &b.parent_hash).skip(to_skip);

        let latest_block_hash = latest_block_in_storage.map(|b| b.hash);
        let all_blocks_hashes = latest_block_hash
            .iter()
            .chain(new_blocks.iter().map(|b| &b.hash));

        let inconsistency = all_blocks_hashes
            .zip(new_blocks_parent_hashes)
            .enumerate()
            .find(|(_, (block_hash, next_block_parent))| block_hash != next_block_parent);

        match inconsistency {
            Some((0, _)) if latest_block_hash.is_some() => Err(ChainError::InconsistentStorage),
            Some(_) => Err(ChainError::InconsistentSequence),
            None => Ok(()),
        }
    }

    /// Processes result of blocks sequnce validation:
    /// - If error is in blocks sequence, do nothing
    /// - If error in storage, discards all blocks after the safe block.
    async fn process_validation_error(&self, validation_error: &ChainError) -> anyhow::Result<()> {
        match validation_error {
            ChainError::InconsistentSequence => {
                log::warn!("inconsistent blocks sequnce fetched");
            }
            ChainError::InconsistentStorage => {
                // Discard all blocks after the safe blocks
                let first_block_to_discard = self
                    .blockchain
                    .get_block_info()
                    .await?
                    .map(|info| info.safe_block_number + 1)
                    .unwrap_or_default();

                log::warn!("Discarding blockchain tail starting with {first_block_to_discard}");

                self.blockchain
                    .discard_blocks_from(first_block_to_discard, "inconsistent")
                    .await?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
enum ChainError {
    #[error("inconsistent block in storage")]
    InconsistentStorage,
    #[error("inconsistent block in new blocks sequence")]
    InconsistentSequence,
}

#[cfg(test)]
mod tests {
    use did::keccak;
    use ethereum_json_rpc_client::reqwest::ReqwestClient;

    use super::*;

    #[test]
    fn test_validate_chain_without_blocks_in_storage() {
        let latest_block_in_storage = Option::<did::Block<did::H256>>::None;
        let sequence = generate_valid_blocks_sequence(10, did::H256::default());
        BlockExtractor::<ReqwestClient>::validate_chain(latest_block_in_storage, &sequence)
            .unwrap();
    }

    #[test]
    fn test_validate_chain_with_block_in_storage() {
        let block = generate_valid_blocks_sequence(1, did::H256::default())
            .pop()
            .unwrap();
        let hash = block.hash.clone();
        let latest_block_in_storage = Some(block);
        let sequence = generate_valid_blocks_sequence(10, hash);
        BlockExtractor::<ReqwestClient>::validate_chain(latest_block_in_storage, &sequence)
            .unwrap();
    }

    #[test]
    fn test_validate_chain_inconsistence_storage() {
        let block = generate_valid_blocks_sequence(1, did::H256::default())
            .pop()
            .unwrap();
        let latest_block_in_storage = Some(block);
        let invalid_parent_hash = keccak::keccak_hash(&[1, 2, 3]);
        let sequence = generate_valid_blocks_sequence(10, invalid_parent_hash);
        let err =
            BlockExtractor::<ReqwestClient>::validate_chain(latest_block_in_storage, &sequence)
                .unwrap_err();
        assert!(matches!(err, ChainError::InconsistentStorage))
    }

    #[test]
    fn test_validate_chain_inconsistence_sequence() {
        let block = generate_valid_blocks_sequence(1, did::H256::default())
            .pop()
            .unwrap();
        let hash = block.hash.clone();
        let latest_block_in_storage = Some(block);
        let mut sequence = generate_valid_blocks_sequence(10, hash);

        // break the sequnce
        sequence[5].parent_hash = keccak::keccak_hash(&[1, 2, 3, 4]);

        let err =
            BlockExtractor::<ReqwestClient>::validate_chain(latest_block_in_storage, &sequence)
                .unwrap_err();
        assert!(matches!(err, ChainError::InconsistentSequence))
    }

    fn generate_valid_blocks_sequence(
        len: usize,
        parent_hash: did::H256,
    ) -> Vec<did::Block<did::H256>> {
        if len == 0 {
            return vec![];
        }

        let mut blocks = (0..len)
            .map(|idx| did::Block {
                number: (idx as u64).into(),
                hash: keccak::keccak_hash(&idx.to_be_bytes()),
                ..Default::default()
            })
            .collect::<Vec<_>>();

        blocks[0].parent_hash = parent_hash;

        for i in 1..blocks.len() {
            blocks[i].parent_hash = blocks[i - 1].hash.clone();
        }

        blocks
    }
}
