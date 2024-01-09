use std::sync::Arc;

use ethereum_json_rpc_client::reqwest::ReqwestClient;
use ethereum_json_rpc_client::EthJsonRcpClient;
use ethers_core::types::BlockNumber;
use itertools::Itertools;
use tokio::time::Duration;

use crate::storage_clients::BlockChainDB;

/// Extracts blocks from an EVMC and stores them in a database
pub struct BlockExtractor {
    rpc_url: String,
    request_time_out_secs: u64,
    rpc_batch_size: usize,
    blockchain: Arc<dyn BlockChainDB>,
}

impl BlockExtractor {
    pub fn new(
        rpc_url: String,
        request_time_out_secs: u64,
        rpc_batch_size: usize,
        blockchain: Arc<dyn BlockChainDB>,
    ) -> Self {
        Self {
            rpc_url,
            blockchain,
            rpc_batch_size,
            request_time_out_secs,
        }
    }

    /// Returns the latest block number in the EVMC
    pub async fn latest_block_number(&self) -> anyhow::Result<u64> {
        let rpc_url = &self.rpc_url;
        let client = EthJsonRcpClient::new(ReqwestClient::new(rpc_url.to_string()));
        client.get_block_number().await
    }

    /// Returns the latest block number stored in the database
    pub async fn latest_block_number_stored(&self) -> anyhow::Result<u64> {
        self.blockchain.get_latest_block_number().await
    }

    /// Collects blocks from the EVMC and stores them in the database
    pub async fn collect_blocks(
        &mut self,
        from_block_inclusive: u64,
        to_block_inclusive: u64,
    ) -> anyhow::Result<()> {
        log::info!(
            "Getting blocks from {} to {}",
            from_block_inclusive,
            to_block_inclusive
        );

        let rpc_url = &self.rpc_url;
        let client = Arc::new(EthJsonRcpClient::new(ReqwestClient::new(
            rpc_url.to_string(),
        )));

        let request_time_out_secs = self.request_time_out_secs;
        let batch_size = self.rpc_batch_size;

        for blocks_batch in &(from_block_inclusive..=to_block_inclusive).chunks(batch_size) {
            let block_numbers = blocks_batch
                .into_iter()
                .map(|block| BlockNumber::Number(block.into()))
                .collect::<Vec<_>>();

            let evm_blocks = tokio::time::timeout(
                Duration::from_secs(request_time_out_secs),
                client.get_full_blocks_by_number(block_numbers, batch_size),
            )
            .await??;

            let mut receipts_tasks = vec![];
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
                .insert_blocks_and_receipts(&evm_blocks, &all_evm_receipts)
                .await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage_clients::hashmap::HashMapBlockchain;

    #[tokio::test]
    async fn test_collect_blocks() {
        let blockchain = Arc::<HashMapBlockchain>::default();
        let rpc_url = "https://testnet.bitfinity.network".to_string();
        let request_time_out_secs = 10;
        let rpc_batch_size = 50;
        let mut extractor =
            BlockExtractor::new(rpc_url, request_time_out_secs, rpc_batch_size, blockchain);

        let end_block = extractor.latest_block_number().await.unwrap();
        let start_block = end_block - 10;
        let block_range = start_block..=end_block;

        for block_number in block_range {
            println!("Processing block number: {}", block_number);
        }
        println!("Getting blocks from {} to {}", start_block, end_block);

        let result = extractor.collect_blocks(start_block, end_block).await;

        if let Err(e) = &result {
            println!("Error: {:?}", e);
        }

        assert!(result.is_ok());

        let latest_block_num = extractor
            .blockchain
            .get_block_by_number(end_block)
            .await
            .unwrap()
            .number
            .unwrap();
        assert_eq!(end_block, latest_block_num.as_u64());
    }
}
