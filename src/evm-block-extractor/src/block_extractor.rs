use std::sync::Arc;

use ethereum_json_rpc_client::reqwest::ReqwestClient;
use ethereum_json_rpc_client::EthJsonRcpClient;
use itertools::Itertools;
use tokio::time::Duration;

use crate::database::DatabaseClient;

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
    pub async fn collect_blocks(
        &mut self,
        from_block_inclusive: u64,
        to_block_inclusive: u64,
    ) -> anyhow::Result<(u64, u64)> {
        log::info!(
            "Getting blocks from {:?} to {}",
            from_block_inclusive,
            to_block_inclusive
        );

        let client = self.client.clone();

        let request_time_out_secs = self.request_time_out_secs;
        let batch_size = self.rpc_batch_size;

        for blocks_batch in &(from_block_inclusive..=to_block_inclusive).chunks(batch_size) {
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
                    client
                        .get_tx_execution_results_by_hash(tx_hashes, batch_size)
                        .await
                });

                receipts_tasks.push(receipts_task);
            }

            let evm_receipts = futures::future::join_all(receipts_tasks).await;

            let mut all_evm_receipts = vec![];
            for receipts in evm_receipts {
                match receipts {
                    Ok(Ok(mut receipts)) => all_evm_receipts.append(&mut receipts),
                    Ok(Err(e)) => {
                        log::warn!("Error getting receipts: {:?}. The process will not be stopped but there will be missing receipts in the DB", e);
                    }
                    Err(e) => {
                        log::warn!("Error getting receipts: {:?}. The process will not be stopped but there will be missing receipts in the DB", e);
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
                .insert_block_data(&blocks, &all_evm_receipts, &all_transactions)
                .await?;
        }

        Ok((from_block_inclusive, to_block_inclusive))
    }
}
