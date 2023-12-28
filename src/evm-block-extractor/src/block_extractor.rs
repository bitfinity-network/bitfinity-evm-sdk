use crate::storage_clients::BlockChainDB;
use std::sync::Arc;
use tokio::time::{self, sleep, Duration};

use ethereum_json_rpc_client::reqwest::ReqwestClient;
use ethereum_json_rpc_client::EthJsonRcpClient;
use ethers_core::types::{Block, BlockNumber, Transaction};
use tokio::sync::{Mutex, Semaphore};

use tokio::task::JoinHandle;
use tokio::time::timeout;

pub struct BlockExtractor {
    rpc_url: String,
    request_time_out_secs: u64,
    pub blockchain: Arc<Mutex<Box<dyn BlockChainDB + Send + Sync>>>,
}

impl BlockExtractor {
    pub fn new(
        rpc_url: String,
        request_time_out_secs: u64,
        blockchain: Box<dyn BlockChainDB + Send + Sync>,
    ) -> Self {
        Self {
            rpc_url,
            blockchain: Arc::new(Mutex::new(blockchain)),
            request_time_out_secs,
        }
    }

    async fn latest_block_number(&self) -> anyhow::Result<u64> {
        let rpc_url = &self.rpc_url;
        let client = EthJsonRcpClient::new(ReqwestClient::new(rpc_url.to_string()));
        client.get_block_number().await
    }

    async fn collect_blocks(
        &mut self,
        start_block: u64,
        end_block: u64,
        max_no_of_requests: usize,
    ) -> anyhow::Result<()> {
        let rpc_url = &self.rpc_url;
        let mut tasks = Vec::new();
        let mut failure_count = 0;
        let delay = Duration::from_secs(1) / max_no_of_requests as u32;
        let semaphore = Arc::new(Semaphore::new(max_no_of_requests));

        for block_number_u64 in start_block..=end_block {
            let request_time_out_secs = self.request_time_out_secs;

            let client = EthJsonRcpClient::new(ReqwestClient::new(rpc_url.to_string()));

            let blockchain = Arc::clone(&self.blockchain);

            let permit = Arc::clone(&semaphore).acquire_owned().await;

            let task: JoinHandle<anyhow::Result<()>> = tokio::spawn(async move {
                let block_number = BlockNumber::Number(block_number_u64.into());

                //throttle the requests
                sleep(delay).await;

                let result = timeout(
                    Duration::from_secs(request_time_out_secs),
                    client.get_full_block_by_number(block_number),
                )
                .await;

                match result {
                    Ok(Ok(block)) => {
                        // blockchain.lock().unwrap().insert_block(block).await?;

                        let mut blockchain = blockchain.lock().await;

                        blockchain.insert_block(block).await?;
                    }
                    Ok(Err(e)) => {
                        println!("Failed to get block {}: {:?}", block_number_u64, e);
                        failure_count += 1;
                    }
                    Err(e) => {
                        println!("Request for block {} timed out: {:?}", block_number_u64, e);
                        failure_count += 1;
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
        println!("Number of failures: {}", failure_count);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::BlockExtractor;
    use crate::storage_clients::hashmap::HashMapBlockchain;

    #[tokio::test]
    async fn test_collect_blocks() {
        let blockchain = Box::new(HashMapBlockchain::new());
        let rpc_url = "https://testnet.bitfinity.network".to_string();
        let request_time_out_secs = 10;
        let mut extractor = BlockExtractor::new(rpc_url, request_time_out_secs, blockchain);

        let end_block = extractor.latest_block_number().await.unwrap();
        let start_block = end_block - 1000;
        let max_requests = 50;
        println!("Getting blocks from {} to {}", start_block, end_block);

        let result = extractor
            .collect_blocks(start_block, end_block, max_requests)
            .await;
        if let Err(e) = &result {
            println!("Error: {:?}", e);
        }
        assert!(result.is_ok());
        // let latest_block_num = extractor.blockchain.lock().unwrap().get_block_by_number(end_block).unwrap().number.unwrap();
        // assert_eq!(end_block, latest_block_num.as_u64());
    }
}