/// Get blocks by number
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

    trace!(
        "sending request: {}",
        serde_json::to_string(&requests).unwrap()
    );

    let response = reqwest::Client::new()
        .post(url)
        .json(&requests)
        .send()
        .await
        .context("failed to get blocks")?
        .json::<Value>()
        .await
        .context("failed to get blocks")?;

    log::trace!("response: {}", response);

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

/// Get receipt by number
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

        log::trace!(
            "sending request: {}",
            serde_json::to_string(&requests).unwrap()
        );

        let response = reqwest::Client::new()
            .post(url)
            .json(&requests)
            .send()
            .await
            .context("failed to get transaction receipt")?
            .json::<Value>()
            .await
            .context("failed to get transaction receipt")?;

        log::trace!("response: {}", response);

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

pub async fn get_block_number(url: &str) -> anyhow::Result<u64> {
    let request: serde_json::Value =
        json!({"jsonrpc":"2.0", "method":"eth_blockNumber", "params":[], "id":1});

    log::trace!("sending request: {}", request);

    let response = reqwest::Client::new()
        .post(url)
        .json(&request)
        .send()
        .await
        .context("failed to get block number")?
        .json::<Value>()
        .await
        .context("failed to get block number")?;

    log::trace!("response: {}", response);
}
