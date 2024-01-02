use std::sync::Arc;

use ethereum_json_rpc_client::reqwest::ReqwestClient;
use ethereum_json_rpc_client::EthJsonRcpClient;
use ethers_core::types::BlockNumber;
use tokio::sync::{Mutex, Semaphore};
use tokio::task::JoinHandle;
use tokio::time::Duration;

use crate::storage_clients::BlockChainDB;

pub struct BlockExtractor {
    rpc_url: String,
    request_time_out_secs: u64,
    rpc_batch_size: u64,
    pub blockchain: Arc<Mutex<Box<dyn BlockChainDB>>>,
}

impl BlockExtractor {
    pub fn new(
        rpc_url: String,
        request_time_out_secs: u64,
        rpc_batch_size: u64,
        blockchain: Box<dyn BlockChainDB>,
    ) -> Self {
        Self {
            rpc_url,
            blockchain: Arc::new(Mutex::new(blockchain)),
            rpc_batch_size,
            request_time_out_secs,
        }
    }

    pub async fn latest_block_number(&self) -> anyhow::Result<u64> {
        let rpc_url = &self.rpc_url;
        let client = EthJsonRcpClient::new(ReqwestClient::new(rpc_url.to_string()));
        client.get_block_number().await
    }

    pub async fn collect_blocks(
        &mut self,
        blocks: impl Iterator<Item = u64>,
        max_no_of_requests: usize,
    ) -> anyhow::Result<()> {
        let rpc_url = &self.rpc_url;
        let mut tasks = Vec::new();
        let delay = Duration::from_secs(1) / max_no_of_requests as u32;
        let semaphore = Arc::new(Semaphore::new(max_no_of_requests));
        let batch_size = self.rpc_batch_size as usize;

        let client = EthJsonRcpClient::new(ReqwestClient::new(rpc_url.to_string()));

        for block_number_u64 in blocks {
            let request_time_out_secs = self.request_time_out_secs;
            let client = client.clone();
            let blockchain = Arc::clone(&self.blockchain);

            let permit = Arc::clone(&semaphore).acquire_owned().await?;

            let task: JoinHandle<anyhow::Result<()>> = tokio::spawn(async move {
                let block_number = BlockNumber::Number(block_number_u64.into());

                //throttle the requests
                tokio::time::sleep(delay).await;

                let result = tokio::time::timeout(
                    Duration::from_secs(request_time_out_secs),
                    client.get_full_block_by_number(block_number),
                )
                .await;

                log::info!("block result: {:?}", result);

                match result {
                    Ok(Ok(block)) => {
                        let mut blockchain = blockchain.lock().await;

                        blockchain.insert_block(&block).await?;

                        let transactions = block.transactions;
                        for chunk in transactions.chunks(batch_size) {
                            let tx_hashes = chunk.iter().map(|tx| tx.hash).collect::<Vec<_>>();

                            let receipts =
                                client.get_receipts_by_hash(tx_hashes, batch_size).await?;

                            blockchain.insert_receipts(&receipts).await?;
                        }
                    }
                    Ok(Err(e)) => {
                        println!("Failed to get block {}: {:?}", block_number_u64, e);
                    }
                    Err(e) => {
                        println!("Request for block {} timed out: {:?}", block_number_u64, e);
                    }
                }

                drop(permit);
                Ok(())
            });

            tasks.push(task);
        }

        for task in tasks {
            let _ = task.await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::BlockExtractor;
    use crate::storage_clients::hashmap::HashMapBlockchain;

    #[tokio::test]
    async fn test_collect_blocks() {
        let blockchain = Box::<HashMapBlockchain>::default();
        let rpc_url = "https://testnet.bitfinity.network".to_string();
        let request_time_out_secs = 10;
        let rpc_batch_size = 50;
        let mut extractor =
            BlockExtractor::new(rpc_url, request_time_out_secs, rpc_batch_size, blockchain);

        let end_block = extractor.latest_block_number().await.unwrap();
        let start_block = end_block - 10;
        let max_requests = 50;
        let block_range = start_block..=end_block;

        for block_number in block_range {
            println!("Processing block number: {}", block_number);
        }
        println!("Getting blocks from {} to {}", start_block, end_block);

        let result = extractor
            .collect_blocks(start_block..=end_block, max_requests)
            .await;

        if let Err(e) = &result {
            println!("Error: {:?}", e);
        }

        assert!(result.is_ok());

        let latest_block_num = extractor
            .blockchain
            .lock()
            .await
            .get_block_by_number(end_block)
            .await
            .unwrap()
            .number
            .unwrap();
        assert_eq!(end_block, latest_block_num.as_u64());
    }
}
