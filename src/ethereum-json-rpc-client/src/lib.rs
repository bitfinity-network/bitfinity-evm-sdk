use std::future::Future;
use std::pin::Pin;

use anyhow::Context;
use ethers_core::types::{Block, BlockNumber, Transaction, TransactionReceipt, H256, U64};
use itertools::Itertools;
use jsonrpc_core::{Call, Id, MethodCall, Output, Params, Request, Response, Version};
use serde::de::DeserializeOwned;

#[cfg(feature = "reqwest")]
pub mod reqwest;

#[cfg(feature = "ic-canister-client")]
pub mod canister_client;

pub mod http_outcall;

const ETH_CHAIN_ID_METHOD: &str = "eth_chainId";
const ETH_GET_BLOCK_BY_NUMBER_METHOD: &str = "eth_getBlockByNumber";
const ETH_BLOCK_NUMBER_METHOD: &str = "eth_blockNumber";
const ETH_GET_TRANSACTION_RECEIPT_METHOD: &str = "eth_getTransactionReceipt";
const ETH_SEND_RAW_TRANSACTION_METHOD: &str = "eth_sendRawTransaction";

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

pub trait Client: Clone + Send + Sync {
    /// Send RPC request.
    ///
    fn send_rpc_request(
        &self,
        request: Request,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<Response>> + Send>>;
}
