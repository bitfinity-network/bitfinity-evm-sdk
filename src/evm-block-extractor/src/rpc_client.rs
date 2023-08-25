use anyhow::Context;
use ethers_core::types::{Block, BlockNumber, Transaction};
use serde_json::{json, Value};

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
        .context("Failed to get transaction receipt")?
        .json::<Value>()
        .await
        .context("Failed to get transaction receipt")?;

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
