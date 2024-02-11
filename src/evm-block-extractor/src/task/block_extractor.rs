use std::sync::Arc;

use ethereum_json_rpc_client::reqwest::ReqwestClient;
use ethereum_json_rpc_client::EthJsonRcpClient;
use ethers_core::types::BlockNumber;
use log::*;
use tokio::time::Duration;

use crate::config::ExtractorArgs;
use crate::database::{AccountBalance, CertifiedBlock, DatabaseClient};
use crate::task::with_retry;

/// Starts the block extractor process
pub async fn start_extractor(
    config: ExtractorArgs,
    db_client: Arc<dyn DatabaseClient>,
    evm_client: Arc<EthJsonRcpClient<ReqwestClient>>,
) -> anyhow::Result<()> {
    let earliest_block = evm_client
        .get_block_by_number(BlockNumber::Earliest)
        .await?;

    db_client
        .init(Some(earliest_block.into()), config.reset_db_on_state_change)
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
        .collect_blocks(start_block.map(|b| b + 1).unwrap_or_default(), end_block)
        .await?;

    extractor.collect_last_certified_block().await?;

    Ok(())
}

/// Extracts blocks from an EVMC and stores them in a database
pub struct BlockExtractor {
    client: Arc<EthJsonRcpClient<ReqwestClient>>,
    request_time_out_secs: u64,
    rpc_batch_size: usize,
    blockchain: Arc<dyn DatabaseClient>,
}

impl BlockExtractor {
    pub fn new(
        client: Arc<EthJsonRcpClient<ReqwestClient>>,
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
    pub async fn collect_blocks(
        &mut self,
        from_block_inclusive: u64,
        to_block_inclusive: u64,
    ) -> anyhow::Result<(u64, u64)> {
        self.collect_chain_id().await?;
        self.collect_genesis_balances().await?;

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
                .map(|block| ethers_core::types::BlockNumber::Number(block.into()));

            let evm_blocks = tokio::time::timeout(
                Duration::from_secs(request_time_out_secs),
                client.get_full_blocks_by_number(block_numbers, batch_size),
            )
            .await??;

            let mut receipts_tasks = vec![];

            let all_transactions = evm_blocks
                .iter()
                .flat_map(|block| &block.transactions)
                .cloned()
                .collect::<Vec<_>>();

            let blocks = evm_blocks
                .into_iter()
                .map(|block| block.into())
                .collect::<Vec<ethers_core::types::Block<ethers_core::types::H256>>>();

            for block in &blocks {
                let tx_hashes = block.transactions.clone();
                let client = client.clone();

                let receipts_task = tokio::spawn(async move {
                    with_retry(
                        "get exe results from evm",
                        Duration::from_secs(1),
                        4,
                        || async {
                            client
                                .get_tx_execution_results_by_hash(tx_hashes.clone(), batch_size)
                                .await
                        },
                    )
                    .await
                });

                receipts_tasks.push(receipts_task);
            }

            let exe_results = futures::future::join_all(receipts_tasks).await;

            let mut all_exe_results = vec![];
            for exe_results in exe_results {
                match exe_results {
                    Ok(Ok(mut exe_results)) => all_exe_results.append(&mut exe_results),
                    Ok(Err(e)) => {
                        warn!("Error getting receipts: {:?}. The process will not be stopped but there will be missing receipts in the DB", e);
                    }
                    Err(e) => {
                        warn!("Error getting receipts: {:?}. The process will not be stopped but there will be missing receipts in the DB", e);
                    }
                }
            }

            let blocks = blocks
                .into_iter()
                .map(|block| block.into())
                .collect::<Vec<did::Block<did::H256>>>();

            let all_transactions = all_transactions
                .into_iter()
                .map(|tx| tx.into())
                .collect::<Vec<did::Transaction>>();

            self.blockchain
                .insert_block_data(&blocks, &all_exe_results, &all_transactions)
                .await?;
        }

        Ok((from_block_inclusive, to_block_inclusive))
    }

    /// Collects last certified block
    async fn collect_last_certified_block(&self) -> anyhow::Result<()> {
        const JSON_RPC_METHOD_LAST_CERTIFIED_BLOCK: &str = "ic_getLastCertifiedBlock";

        let certified_block = self
            .client
            .single_request::<CertifiedBlock>(
                JSON_RPC_METHOD_LAST_CERTIFIED_BLOCK.to_string(),
                jsonrpc_core::Params::Array(vec![]),
                jsonrpc_core::Id::Null,
            )
            .await?;
        self.blockchain
            .insert_certified_block_data(certified_block)
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
                    .map(|(address, balance)| AccountBalance {
                        address: address.into(),
                        balance: balance.into(),
                    })
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
