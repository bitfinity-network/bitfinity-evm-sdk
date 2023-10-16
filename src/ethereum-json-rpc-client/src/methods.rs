use ethers_core::types::{Block, BlockNumber, Transaction, TransactionReceipt, H256, U64};
use jsonrpc_core::{Id, Params};

use crate::request::{batch_request, single_request};

const GET_BLOCK_BY_NUMBER_METHOD: &str = "eth_getBlockByNumber";
const GET_BLOCK_NUMBER_METHOD: &str = "eth_blockNumber";
const GET_TRANSACTION_RECEIPT_METHOD: &str = "eth_getTransactionReceipt";

macro_rules! make_params_array {
    ($($items:expr),*) => {
        Params::Array(vec![$(serde_json::to_value($items)?, )*])
    };
}

/// Returns block with transaction hashes by number
pub async fn get_block_by_number(url: &str, block: BlockNumber) -> anyhow::Result<Block<H256>> {
    single_request(
        url,
        GET_BLOCK_BY_NUMBER_METHOD.to_string(),
        make_params_array!(block, false),
        // For some reason some JSON RPC services fail to parse requests with null id
        Id::Str("get_block_by_number".to_string()),
    )
    .await
}

/// Returns full block by number
pub async fn get_full_block_by_number(
    url: &str,
    block: BlockNumber,
) -> anyhow::Result<Block<Transaction>> {
    single_request(
        url,
        GET_BLOCK_BY_NUMBER_METHOD.to_string(),
        make_params_array!(block, true),
        // For some reason some JSON RPC services fail to parse requests with null id
        Id::Str("get_full_block_by_number".to_string()),
    )
    .await
}

/// Returns full blocks by number
pub async fn get_full_blocks_by_number(
    url: &str,
    blocks: impl IntoIterator<Item = BlockNumber>,
    max_batch_size: usize,
) -> anyhow::Result<Vec<Block<Transaction>>> {
    let params = blocks
        .into_iter()
        .enumerate()
        .map(|(index, block_number)| -> anyhow::Result<(Params, Id)> {
            Ok((make_params_array!(block_number, true), Id::Num(index as _)))
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    batch_request(
        url,
        GET_BLOCK_BY_NUMBER_METHOD.to_string(),
        params,
        max_batch_size,
    )
    .await
}

/// Get receipt by number
pub async fn get_receipts_by_hash(
    url: &str,
    hashes: impl IntoIterator<Item = H256>,
    max_batch_size: usize,
) -> anyhow::Result<Vec<TransactionReceipt>> {
    let params = hashes
        .into_iter()
        .map(|hash| -> anyhow::Result<(Params, Id)> {
            Ok((make_params_array!(hash), Id::Str(hash.to_string())))
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    batch_request(
        url,
        GET_TRANSACTION_RECEIPT_METHOD.to_string(),
        params,
        max_batch_size,
    )
    .await
}

/// Returns chain block number
pub async fn get_block_number(url: &str) -> anyhow::Result<u64> {
    single_request::<U64>(
        url,
        GET_BLOCK_NUMBER_METHOD.to_string(),
        make_params_array!(),
        Id::Null,
    )
    .await
    .map(|v| v.as_u64())
}
