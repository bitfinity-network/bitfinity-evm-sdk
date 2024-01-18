use std::collections::HashMap;
use std::sync::Arc;

use ethers_core::types::{Block, Transaction, TransactionReceipt, H256};
use tokio::sync::Mutex;

use super::DatabaseClient;

#[derive(Clone, Default)]
pub struct InMemoryDbClient {
    pub blocks: Arc<Mutex<HashMap<u64, Block<H256>>>>,
    pub receipts: Arc<Mutex<HashMap<H256, TransactionReceipt>>>,
    pub transactions: Arc<Mutex<HashMap<H256, Transaction>>>,
}

#[async_trait::async_trait]
impl DatabaseClient for InMemoryDbClient {
    async fn init(&self) -> anyhow::Result<()> {
        Ok(())
    }

    async fn get_block_by_number(
        &self,
        block: u64,
        include_transactions: bool,
    ) -> anyhow::Result<serde_json::Value> {
        match self.blocks.lock().await.get(&block) {
            Some(block) => {
                if include_transactions {
                    let mut transactions = Vec::new();
                    for transaction in self.transactions.lock().await.values() {
                        if transaction.block_number.expect("should be there").as_u64()
                            == block.number.expect("should be there").as_u64()
                        {
                            transactions.push(transaction.clone());
                        }
                    }

                    let full_block = block.clone().into_full_block(transactions);

                    Ok(serde_json::to_value(full_block)?)
                } else {
                    Ok(serde_json::to_value(block)?)
                }
            }
            None => Err(anyhow::anyhow!("Block not found")),
        }
    }

    async fn insert_block_data(
        &self,
        block: &[Block<H256>],
        receipts: &[TransactionReceipt],
        transactions: &[Transaction],
    ) -> anyhow::Result<()> {
        let mut receipts_map = self.receipts.lock().await;
        for receipt in receipts {
            receipts_map.insert(receipt.transaction_hash, receipt.clone());
        }

        let mut blocks_map = self.blocks.lock().await;
        for block in block {
            blocks_map.insert(block.number.unwrap().as_u64(), block.clone());
        }

        let mut transactions_map = self.transactions.lock().await;

        for txn in transactions {
            transactions_map.insert(txn.hash, txn.clone());
        }

        Ok(())
    }

    async fn get_transaction_receipt(&self, tx_hash: H256) -> anyhow::Result<TransactionReceipt> {
        match self.receipts.lock().await.get(&tx_hash) {
            Some(receipt) => Ok(receipt.clone()),
            None => Err(anyhow::anyhow!("Receipt not found")),
        }
    }

    async fn get_latest_block_number(&self) -> anyhow::Result<Option<u64>> {
        let block_map = self.blocks.lock().await;
        if block_map.is_empty() {
            return Ok(None);
        }
        let mut max_block_number = 0;
        for block_number in block_map.keys() {
            if *block_number > max_block_number {
                max_block_number = *block_number;
            }
        }
        Ok(Some(max_block_number))
    }

    async fn get_earliest_block_number(&self) -> anyhow::Result<u64> {
        let mut min_block_number = u64::MAX;
        for block_number in self.blocks.lock().await.keys() {
            if *block_number < min_block_number {
                min_block_number = *block_number;
            }
        }
        Ok(min_block_number)
    }
}
