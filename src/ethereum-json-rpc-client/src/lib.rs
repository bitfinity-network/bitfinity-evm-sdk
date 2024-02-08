use std::future::Future;
use std::pin::Pin;

use anyhow::Context;
use did::transaction::StorableExecutionResult;
use ethers_core::types::{
    Block, BlockNumber, Log, Transaction, TransactionReceipt, H160, H256, U256, U64,
};
use itertools::Itertools;
use jsonrpc_core::{Call, Id, MethodCall, Output, Params, Request, Response, Version};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

pub mod http;

#[cfg(feature = "reqwest")]
pub mod reqwest;

#[cfg(feature = "ic-canister-client")]
pub mod canister_client;

#[cfg(feature = "http-outcall")]
pub mod http_outcall;

const ETH_CHAIN_ID_METHOD: &str = "eth_chainId";
const ETH_GET_BLOCK_BY_NUMBER_METHOD: &str = "eth_getBlockByNumber";
const ETH_BLOCK_NUMBER_METHOD: &str = "eth_blockNumber";
const ETH_GET_TRANSACTION_RECEIPT_METHOD: &str = "eth_getTransactionReceipt";
const ETH_SEND_RAW_TRANSACTION_METHOD: &str = "eth_sendRawTransaction";
const ETH_GET_LOGS_METHOD: &str = "eth_getLogs";
const IC_GET_TX_EXECUTION_RESULT_BY_HASH_METHOD: &str = "ic_getExeResultByHash";
const IC_GET_GENESIS_BALANCES: &str = "ic_getGenesisBalances";

/// A client for interacting with an Ethereum node over JSON-RPC.
#[derive(Clone)]
pub struct EthJsonRcpClient<C: Client> {
    client: C,
}

macro_rules! make_params_array {
    ($($items:expr),*) => {
        Params::Array(vec![$(serde_json::to_value($items)?, )*])
    };
}

impl<C: Client> EthJsonRcpClient<C> {
    /// Create a new client.
    ///
    /// # Arguments
    /// * `client` - The canister client.
    pub fn new(client: C) -> Self {
        Self { client }
    }

    /// Returns block with transaction hashes by number
    pub async fn get_block_by_number(&self, block: BlockNumber) -> anyhow::Result<Block<H256>> {
        self.single_request(
            ETH_GET_BLOCK_BY_NUMBER_METHOD.to_string(),
            make_params_array!(block, false),
            // For some reason some JSON RPC services fail to parse requests with null id
            Id::Str("get_block_by_number".to_string()),
        )
        .await
    }

    /// Returns full block by number
    pub async fn get_full_block_by_number(
        &self,
        block: BlockNumber,
    ) -> anyhow::Result<Block<Transaction>> {
        self.single_request(
            ETH_GET_BLOCK_BY_NUMBER_METHOD.to_string(),
            make_params_array!(block, true),
            // For some reason some JSON RPC services fail to parse requests with null id
            Id::Str("get_full_block_by_number".to_string()),
        )
        .await
    }

    /// Returns full blocks by number
    pub async fn get_full_blocks_by_number(
        &self,
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
        self.batch_request(
            ETH_GET_BLOCK_BY_NUMBER_METHOD.to_string(),
            params,
            max_batch_size,
        )
        .await
    }

    /// Get receipt by number
    pub async fn get_receipts_by_hash(
        &self,
        hashes: impl IntoIterator<Item = H256>,
        max_batch_size: usize,
    ) -> anyhow::Result<Vec<TransactionReceipt>> {
        let params = hashes
            .into_iter()
            .map(|hash| -> anyhow::Result<(Params, Id)> {
                Ok((make_params_array!(hash), Id::Str(hash.to_string())))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        self.batch_request(
            ETH_GET_TRANSACTION_RECEIPT_METHOD.to_string(),
            params,
            max_batch_size,
        )
        .await
    }

    /// Get receipt by hash
    pub async fn get_receipt_by_hash(&self, hash: H256) -> anyhow::Result<TransactionReceipt> {
        self.single_request(
            ETH_GET_TRANSACTION_RECEIPT_METHOD.to_string(),
            make_params_array!(hash),
            Id::Str(hash.to_string()),
        )
        .await
    }

    /// Returns chain block number
    pub async fn get_block_number(&self) -> anyhow::Result<u64> {
        self.single_request::<U64>(
            ETH_BLOCK_NUMBER_METHOD.to_string(),
            make_params_array!(),
            Id::Null,
        )
        .await
        .map(|v| v.as_u64())
    }

    /// Returns chain id
    pub async fn get_chain_id(&self) -> anyhow::Result<u64> {
        self.single_request::<U64>(
            ETH_CHAIN_ID_METHOD.to_string(),
            Params::Array(vec![]),
            Id::Str("eth_chainId".to_string()),
        )
        .await
        .map(|v| v.as_u64())
    }

    /// Sends raw transaction and returns transaction hash
    pub async fn send_raw_transaction(&self, transaction: Transaction) -> anyhow::Result<H256> {
        let bytes = transaction.rlp();
        let transaction = format!("0x{}", hex::encode(bytes));

        self.single_request::<H256>(
            ETH_SEND_RAW_TRANSACTION_METHOD.to_string(),
            make_params_array!(transaction),
            Id::Str("send_rawTransaction".to_string()),
        )
        .await
    }

    /// Get EVM logs according to the given parameters.
    pub async fn get_logs(&self, params: EthGetLogsParams) -> anyhow::Result<Vec<Log>> {
        self.single_request(
            ETH_GET_LOGS_METHOD.to_string(),
            make_params_array!(params),
            Id::Str("ETH_GET_LOGS_METHOD".to_string()),
        )
        .await
    }

    /// Returns the transaction execution result by hash
    pub async fn get_tx_execution_result_by_hash(
        &self,
        hash: H256,
    ) -> anyhow::Result<StorableExecutionResult> {
        let transaction = self
            .single_request::<Option<StorableExecutionResult>>(
                IC_GET_TX_EXECUTION_RESULT_BY_HASH_METHOD.to_string(),
                make_params_array!(hash),
                Id::Str(hash.to_string()),
            )
            .await?
            .ok_or_else(|| anyhow::anyhow!("Transaction not found"))?;

        Ok(transaction)
    }

    /// Returns the transaction execution result by hash in batch
    pub async fn get_tx_execution_results_by_hash(
        &self,
        hashes: impl IntoIterator<Item = H256>,
        max_batch_size: usize,
    ) -> anyhow::Result<Vec<StorableExecutionResult>> {
        let params = hashes
            .into_iter()
            .enumerate()
            .map(|(index, hash)| -> anyhow::Result<(Params, Id)> {
                Ok((make_params_array!(hash), Id::Num(index as _)))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        Ok(self
            .batch_request::<Option<StorableExecutionResult>>(
                IC_GET_TX_EXECUTION_RESULT_BY_HASH_METHOD.to_string(),
                params,
                max_batch_size,
            )
            .await?
            .into_iter()
            .flatten()
            .collect())
    }

    /// Returns the genesis accounts
    pub async fn get_genesis_balances(&self) -> anyhow::Result<Vec<(H160, U256)>> {
        self.single_request(
            IC_GET_GENESIS_BALANCES.to_string(),
            make_params_array!(),
            Id::Str(IC_GET_GENESIS_BALANCES.to_string()),
        )
        .await
    }

    /// Performs a request.
    pub async fn request(&self, request: Request) -> anyhow::Result<Response> {
        self.client.send_rpc_request(request).await
    }

    /// Performs a single request.
    pub async fn single_request<R: DeserializeOwned>(
        &self,
        method: String,
        params: Params,
        id: Id,
    ) -> anyhow::Result<R> {
        let request = Request::Single(Call::MethodCall(MethodCall {
            jsonrpc: Some(Version::V2),
            method,
            params,
            id,
        }));

        let response = self.client.send_rpc_request(request).await?;

        match response {
            Response::Single(response) => match response {
                Output::Success(result) => {
                    serde_json::from_value(result.result).context("failed to deserialize value")
                }
                Output::Failure(err) => Err(anyhow::format_err!("{err:?}")),
            },
            Response::Batch(_) => Err(anyhow::format_err!("unexpected response type: batch")),
        }
    }

    /// Performs a batch request.
    pub async fn batch_request<R: DeserializeOwned>(
        &self,
        method: String,
        params: impl IntoIterator<Item = (Params, Id)>,
        max_batch_size: usize,
    ) -> anyhow::Result<Vec<R>> {
        let mut results = Vec::new();

        let value_from_json = |value| serde_json::from_value::<R>(value);

        // Collect chunks before iteration, otherwise the future won't be `Send`
        let chunks = params
            .into_iter()
            .chunks(max_batch_size)
            .into_iter()
            .map(Iterator::collect::<Vec<_>>)
            .collect::<Vec<_>>();
        for chunk in chunks {
            let method_calls = chunk
                .into_iter()
                .map(|(params, id)| {
                    Call::MethodCall(MethodCall {
                        jsonrpc: Some(Version::V2),
                        method: method.clone(),
                        params,
                        id,
                    })
                })
                .collect::<Vec<_>>();
            let chunk_size = method_calls.len();
            let request = Request::Batch(method_calls);

            let response = self.client.send_rpc_request(request).await?;

            match response {
                Response::Single(response) => match response {
                    Output::Success(result) => {
                        if chunk_size == 1 {
                            results.push(value_from_json(result.result)?);
                        } else {
                            anyhow::bail!(
                                "unexpected number of results: have: 1, expected {chunk_size}"
                            );
                        }
                    }
                    Output::Failure(err) => {
                        anyhow::bail!("{err:?}");
                    }
                },
                Response::Batch(response) => {
                    if chunk_size == response.len() {
                        for resp in response.into_iter() {
                            match resp {
                                Output::Success(resp) => {
                                    results.push(value_from_json(resp.result)?)
                                }
                                Output::Failure(err) => {
                                    anyhow::bail!("{err:?}");
                                }
                            }
                        }
                    } else {
                        anyhow::bail!(
                            "unexpected number of results: have: {}, expected {}",
                            response.len(),
                            chunk_size
                        );
                    }
                }
            }
        }

        Ok(results)
    }
}

/// Parameters to `eth_getLogs`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthGetLogsParams {
    /// Addresses of contracts to filter logs for.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<Vec<H160>>,

    /// Start search logs from this block number.
    #[serde(rename = "fromBlock")]
    pub from_block: BlockNumber,

    /// Finish search logs on this block number.
    #[serde(rename = "toBlock")]
    pub to_block: BlockNumber,

    /// Filter logs by topics.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topics: Option<Vec<Vec<H256>>>,
}

pub trait Client: Clone + Send + Sync {
    /// Send RPC request.
    ///
    fn send_rpc_request(
        &self,
        request: Request,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<Response>> + Send>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eth_get_logs_params_serialization() {
        let get_logs_params = EthGetLogsParams {
            address: Some(vec!["0xb59f67a8bff5d8cd03f6ac17265c550ed8f33907"
                .parse()
                .unwrap()]),
            from_block: BlockNumber::Number(42u64.into()),
            to_block: BlockNumber::Latest,
            topics: Some(vec![
                vec![
                    "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"
                        .parse()
                        .unwrap(),
                ],
                vec![
                    "0x00000000000000000000000000b46c2526e227482e2ebb8f4c69e4674d262e75"
                        .parse()
                        .unwrap(),
                ],
                vec![
                    "0x00000000000000000000000054a2d42a40f51259dedd1978f6c118a0f0eff078"
                        .parse()
                        .unwrap(),
                ],
            ]),
        };

        let json = serde_json::to_string(&get_logs_params).unwrap();

        let expected_json = "{\
            \"address\":[\"0xb59f67a8bff5d8cd03f6ac17265c550ed8f33907\"],\
            \"fromBlock\":\"0x2a\",\
            \"toBlock\":\"latest\",\
            \"topics\":[\
                [\"0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef\"],\
                [\"0x00000000000000000000000000b46c2526e227482e2ebb8f4c69e4674d262e75\"],\
                [\"0x00000000000000000000000054a2d42a40f51259dedd1978f6c118a0f0eff078\"]\
        ]}";
        assert_eq!(json, expected_json);
    }
}
