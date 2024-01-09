use std::sync::Arc;

use anyhow::Context;
use ethereum_json_rpc_client::reqwest::ReqwestClient;
use ethereum_json_rpc_client::EthJsonRcpClient;
use ethers_core::types::{Block, BlockNumber, Transaction, TransactionReceipt, H256};
use itertools::Itertools;
use tokio::sync::{mpsc, Mutex};
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

    pub async fn latest_block_number_stored(&self) -> anyhow::Result<u64> {
        let blockchain = self.blockchain.lock().await;
        blockchain.get_latest_block_number().await
    }

    pub async fn collect_blocks(
        &mut self,
        blocks: impl Iterator<Item = u64>,
        max_no_of_requests: usize,
    ) -> anyhow::Result<()> {
        let rpc_url = &self.rpc_url;
        let client = Arc::new(EthJsonRcpClient::new(ReqwestClient::new(
            rpc_url.to_string(),
        )));
        let request_time_out_secs = self.request_time_out_secs;
        let batch_size = self.rpc_batch_size as usize;

        let (tx, rx) = mpsc::channel::<Vec<H256>>(max_no_of_requests);

        let rx = Arc::new(Mutex::new(rx));

        for block in &blocks.chunks(batch_size) {
            let client_clone = client.clone();
            let tx = tx.clone();

            let block_numbers = block
                .into_iter()
                .map(|block| BlockNumber::Number(block.into()))
                .collect::<Vec<_>>();

            let block_task = tokio::spawn(async move {
                let blocks = tokio::time::timeout(
                    Duration::from_secs(request_time_out_secs),
                    client_clone.get_full_blocks_by_number(block_numbers, batch_size),
                )
                .await??;

                let tx_hashes = blocks
                    .iter()
                    .flat_map(|block| block.transactions.iter().map(|tx| tx.hash))
                    .collect::<Vec<_>>();

                tx.send(tx_hashes).await?;

                anyhow::Result::<Vec<Block<Transaction>>>::Ok(blocks)
            });

            let receipts_task = tokio::spawn({
                let client = client.clone();
                let rx = rx.clone();
                async move {
                    let mut rx = rx.lock().await;
                    let tx_hashes = rx.recv().await.context("Error receiving tx hashes")?;
                    let receipts = client.get_receipts_by_hash(tx_hashes, batch_size).await?;
                    anyhow::Result::<Vec<TransactionReceipt>>::Ok(receipts)
                }
            });

            let (blocks, receipts) = futures::future::join(block_task, receipts_task).await;

            let (blocks, receipts) = (blocks??, receipts??);

            let mut blockchain = self.blockchain.lock().await;

            blockchain
                .insert_blocks_and_receipts(&blocks, &receipts)
                .await?;
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
