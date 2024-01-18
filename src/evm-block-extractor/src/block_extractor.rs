use std::sync::Arc;

use ethereum_json_rpc_client::reqwest::ReqwestClient;
use ethereum_json_rpc_client::EthJsonRcpClient;
use ethers_core::types::BlockNumber;
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
                .map(|block| BlockNumber::Number(block.into()));

            let evm_blocks = tokio::time::timeout(
                Duration::from_secs(request_time_out_secs),
                client.get_full_blocks_by_number(block_numbers, batch_size),
            )
            .await??;

            let mut receipts_tasks = vec![];

            let blocks = evm_blocks
                .iter()
                .map(|block| block.clone().into())
                .collect::<Vec<_>>();

            let all_transactions = evm_blocks
                .iter()
                .flat_map(|block| &block.transactions)
                .cloned()
                .collect::<Vec<_>>();

            for block in &evm_blocks {
                let tx_hashes = block
                    .transactions
                    .iter()
                    .map(|tx| tx.hash)
                    .collect::<Vec<_>>();

                let client = client.clone();
                let receipts_task = tokio::spawn(async move {
                    client.get_receipts_by_hash(tx_hashes, batch_size).await
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

            self.blockchain
                .insert_block_data(&blocks, &all_evm_receipts, &all_transactions)
                .await?;
        }

        Ok((from_block_inclusive, to_block_inclusive))
    }
}

#[cfg(test)]
mod tests {
    use ethers_core::types::{Block, Transaction};

    use super::*;
    use crate::database::in_memory_db_client::InMemoryDbClient;

    #[tokio::test]
    async fn test_collect_blocks() {
        let blockchain = Arc::<InMemoryDbClient>::default();
        let rpc_url = "https://testnet.bitfinity.network".to_string();
        let request_time_out_secs = 10;
        let rpc_batch_size = 10;

        let evm_client = Arc::new(EthJsonRcpClient::new(ReqwestClient::new(rpc_url)));

        let mut extractor = BlockExtractor::new(
            evm_client.clone(),
            request_time_out_secs,
            rpc_batch_size,
            blockchain.clone(),
        );

        let end_block = evm_client.get_block_number().await.unwrap();
        let start_block = end_block - 10;

        println!("Getting blocks from {:?} to {}", start_block, end_block);

        let result = extractor.collect_blocks(start_block, end_block).await;

        if let Err(e) = &result {
            println!("Error: {:?}", e);
        }

        assert!(result.is_ok());

        let latest_block_num: Block<Transaction> = serde_json::from_value(
            blockchain
                .get_block_by_number(end_block, true)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(end_block, latest_block_num.number.unwrap().as_u64());
    }
}
