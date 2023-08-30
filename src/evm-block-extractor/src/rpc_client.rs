use anyhow::Context;
use ethers_core::types::{Block, BlockNumber, Transaction, TransactionReceipt};
use serde_json::{json, Value};

pub const MAX_BATCH_REQUESTS: usize = 5; // Max batch size is 5 in EVM

/// Get Blocks by number
pub async fn get_blocks_by_number(
    url: &str,
    blocks: &[BlockNumber],
) -> anyhow::Result<Vec<Block<Transaction>>> {
    let requests: Vec<serde_json::Value> = blocks
        .iter()
        .enumerate()
        .map(|(id, block_number)| {
            json!({"jsonrpc":"2.0","method":"eth_getBlockByNumber","params":[
        block_number,
        true
    ],"id":id})
        })
        .collect();

    let response = reqwest::Client::new()
        .post(url)
        .json(&requests)
        .send()
        .await
        .context("Failed to get blocks")?
        .json::<Value>()
        .await
        .context("Failed to get blocks")?;

    if !response.is_array() {
        anyhow::bail!("response is not an array");
    }

    let response = response.as_array().unwrap();
    let mut blocks = Vec::with_capacity(response.len());

    for entry in response {
        let block = entry["result"].clone();
        if !block.is_null() {
            blocks.push(
                serde_json::from_value::<Block<Transaction>>(block).context("bad block value")?,
            );
        }
    }

    Ok(blocks)
}

/// Get Blocks by number
pub async fn get_receipts_by_number(
    url: &str,
    block: &Block<Transaction>,
) -> anyhow::Result<Vec<TransactionReceipt>> {
    let mut receipts = Vec::with_capacity(block.transactions.len());

    for transactions in block.transactions.chunks(MAX_BATCH_REQUESTS) {
        let requests: Vec<serde_json::Value> = transactions
            .iter()
            .enumerate()
            .map(|(id, tx)| {
                json!({"jsonrpc":"2.0","method":"eth_getTransactionReceipt","params":[
        tx.hash
    ],"id":id})
            })
            .collect();

        println!("{}", serde_json::to_string(&requests).unwrap());

        let response = reqwest::Client::new()
            .post(url)
            .json(&requests)
            .send()
            .await
            .context("Failed to get transaction receipt")?
            .json::<Value>()
            .await
            .context("Failed to get transaction receipt")?;

        if !response.is_array() {
            anyhow::bail!("response is not an array");
        }

        let response = response.as_array().unwrap();

        for entry in response {
            let receipt = entry["result"].clone();
            if !receipt.is_null() {
                receipts.push(
                    serde_json::from_value::<TransactionReceipt>(receipt)
                        .context("bad receipt value")?,
                );
            }
        }
    }

    Ok(receipts)
}
