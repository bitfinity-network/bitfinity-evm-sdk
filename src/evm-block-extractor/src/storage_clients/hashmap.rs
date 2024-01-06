use std::collections::HashMap;

use ethers_core::types::{Block, Transaction, TransactionReceipt, H256};

use super::BlockChainDB;

#[derive(Clone, Default)]
pub struct HashMapBlockchain {
    pub blocks: HashMap<u64, Block<Transaction>>,
    pub receipts: HashMap<H256, TransactionReceipt>,
}

#[async_trait::async_trait]
impl BlockChainDB for HashMapBlockchain {
    async fn get_blocks_in_range(&self, start: u64, end: u64) -> anyhow::Result<Vec<u64>> {
        let mut blocks_in_range = Vec::new();
        for block_number in start..=end {
            if self.blocks.contains_key(&block_number) {
                blocks_in_range.push(block_number);
            }
        }
        Ok(blocks_in_range)
    }

    async fn get_block_by_number(&self, block: u64) -> anyhow::Result<Block<Transaction>> {
        match self.blocks.get(&block) {
            Some(block) => Ok(block.clone()),
            None => Err(anyhow::anyhow!("Block not found")),
        }
    }
    async fn insert_block(&mut self, block: &Block<Transaction>) -> anyhow::Result<()> {
        self.blocks
            .insert(block.number.unwrap().as_u64(), block.clone());
        Ok(())
    }

    async fn insert_receipts(&mut self, receipts: &[TransactionReceipt]) -> anyhow::Result<()> {
        for receipt in receipts {
            self.receipts
                .insert(receipt.transaction_hash, receipt.clone());
        }
        Ok(())
    }

    async fn get_transaction_receipt(&self, tx_hash: H256) -> anyhow::Result<TransactionReceipt> {
        match self.receipts.get(&tx_hash) {
            Some(receipt) => Ok(receipt.clone()),
            None => Err(anyhow::anyhow!("Receipt not found")),
        }
    }

    async fn get_latest_block_number(&self) -> anyhow::Result<u64> {
        let mut max_block_number = 0;
        for block_number in self.blocks.keys() {
            if *block_number > max_block_number {
                max_block_number = *block_number;
            }
        }
        Ok(max_block_number)
    }

    async fn get_earliest_block_number(&self) -> anyhow::Result<u64> {
        let mut min_block_number = u64::MAX;
        for block_number in self.blocks.keys() {
            if *block_number < min_block_number {
                min_block_number = *block_number;
            }
        }
        Ok(min_block_number)
    }
}
