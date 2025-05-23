#[cfg(feature = "ic-canister-client")]
pub mod canister_client;
mod error;
#[cfg(feature = "http-outcall")]
pub mod http_outcall;
#[cfg(feature = "reqwest")]
pub mod reqwest;

use std::future::Future;
use std::pin::Pin;

use alloy::consensus::TxEnvelope;
use alloy::rpc::types::{Log, TransactionRequest};
pub use did::certified::CertifiedResult;
use did::evm_state::EvmGlobalState;
use did::rpc::id::Id;
use did::rpc::params::Params;
use did::rpc::request::{Request, RpcRequest};
use did::rpc::response::{Response, RpcResponse};
use did::rpc::version::Version;
pub use did::transaction::StorableExecutionResult;
use did::{
    Block, BlockConfirmationData, BlockConfirmationResult, BlockNumber, BlockchainBlockInfo, H160,
    H256, Transaction, TransactionReceipt, U64, U256,
};
use itertools::Itertools;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use self::error::{JsonRpcError, JsonRpcResult};

const ETH_CHAIN_ID_METHOD: &str = "eth_chainId";
const ETH_GET_BALANCE_METHOD: &str = "eth_getBalance";
const ETH_GAS_PRICE_METHOD: &str = "eth_gasPrice";
const ETH_GET_CODE_METHOD: &str = "eth_getCode";
const ETH_GET_TRANSACTION_COUNT_METHOD: &str = "eth_getTransactionCount";
const ETH_GET_BLOCK_BY_NUMBER_METHOD: &str = "eth_getBlockByNumber";
const ETH_BLOCK_NUMBER_METHOD: &str = "eth_blockNumber";
const ETH_GET_TRANSACTION_RECEIPT_METHOD: &str = "eth_getTransactionReceipt";
const ETH_CALL_METHOD: &str = "eth_call";
const ETH_GET_TRANSACTION_BY_HASH_METHOD: &str = "eth_getTransactionByHash";
const ETH_GET_LOGS_METHOD: &str = "eth_getLogs";
const IC_GET_TX_EXECUTION_RESULT_BY_HASH_METHOD: &str = "ic_getExeResultByHash";
const IC_GET_GENESIS_BALANCES: &str = "ic_getGenesisBalances";
const IC_GET_LAST_CERTIFIED_BLOCK: &str = "ic_getLastCertifiedBlock";
const IC_GET_EVM_GLOBAL_STATE: &str = "ic_getEvmGlobalState";
const IC_GET_BLOCKCHAIN_BLOCK_INFO: &str = "ic_getBlockchainBlockInfo";
const ETH_MAX_PRIORITY_FEE_PER_GAS_METHOD: &str = "eth_maxPriorityFeePerGas";

// List of methods that require an `update` IC query endpoint
const ETH_SEND_RAW_TRANSACTION_METHOD: &str = "eth_sendRawTransaction";
const IC_SEND_CONFIRM_BLOCK: &str = "ic_sendConfirmBlock";

macro_rules! make_params_array {
    ($($items:expr_2021),*) => {
        Params::Array(vec![$(serde_json::to_value($items)?, )*])
    };
}

/// A client for interacting with an Ethereum node over JSON-RPC.
#[derive(Clone)]
pub struct EthJsonRpcClient<C: Client> {
    client: C,
}

impl<C: Client> EthJsonRpcClient<C> {
    /// Create a new client.
    ///
    /// # Arguments
    /// * `client` - The canister client.
    pub fn new(client: C) -> Self {
        Self { client }
    }

    /// Returns block with transaction hashes by number
    pub async fn get_block_by_number(&self, block: BlockNumber) -> JsonRpcResult<Block<H256>> {
        self.single_request(
            ETH_GET_BLOCK_BY_NUMBER_METHOD.to_string(),
            make_params_array!(block, false),
            // For some reason some JSON RPC services fail to parse requests with null id
            Id::String(ETH_GET_BLOCK_BY_NUMBER_METHOD.to_string()),
        )
        .await
    }

    /// Returns full block by number
    pub async fn get_full_block_by_number(
        &self,
        block: BlockNumber,
    ) -> JsonRpcResult<Block<Transaction>> {
        self.single_request(
            ETH_GET_BLOCK_BY_NUMBER_METHOD.to_string(),
            make_params_array!(block, true),
            // For some reason some JSON RPC services fail to parse requests with null id
            Id::String(ETH_GET_BLOCK_BY_NUMBER_METHOD.to_string()),
        )
        .await
    }

    /// Returns full blocks by number
    pub async fn get_full_blocks_by_number(
        &self,
        blocks: impl IntoIterator<Item = BlockNumber>,
        max_batch_size: usize,
    ) -> JsonRpcResult<Vec<Block<Transaction>>> {
        let params = blocks
            .into_iter()
            .enumerate()
            .map(|(index, block_number)| -> JsonRpcResult<(Params, Id)> {
                Ok((
                    make_params_array!(block_number, true),
                    Id::Number(index as _),
                ))
            })
            .collect::<JsonRpcResult<Vec<_>>>()?;
        self.batch_request(ETH_GET_BLOCK_BY_NUMBER_METHOD, params, max_batch_size)
            .await
    }

    /// Get receipt by number
    pub async fn get_receipts_by_hash(
        &self,
        hashes: impl IntoIterator<Item = H256>,
        max_batch_size: usize,
    ) -> JsonRpcResult<Vec<TransactionReceipt>> {
        let params = hashes
            .into_iter()
            .map(|hash| -> JsonRpcResult<(Params, Id)> {
                let id = Id::String(hash.0.to_string());
                Ok((make_params_array!(hash), id))
            })
            .collect::<JsonRpcResult<Vec<_>>>()?;
        self.batch_request(ETH_GET_TRANSACTION_RECEIPT_METHOD, params, max_batch_size)
            .await
    }

    /// Get receipt by hash
    pub async fn get_receipt_by_hash(&self, hash: H256) -> JsonRpcResult<TransactionReceipt> {
        let id = Id::String(hash.0.to_string());
        self.single_request(
            ETH_GET_TRANSACTION_RECEIPT_METHOD.to_string(),
            make_params_array!(hash),
            id,
        )
        .await
    }

    /// Returns chain block number
    pub async fn get_block_number(&self) -> JsonRpcResult<u64> {
        self.single_request::<U64>(
            ETH_BLOCK_NUMBER_METHOD.to_string(),
            make_params_array!(),
            Id::String(ETH_BLOCK_NUMBER_METHOD.to_string()),
        )
        .await
        .map(|v| v.0.to())
    }

    /// Returns chain id
    pub async fn get_chain_id(&self) -> JsonRpcResult<u64> {
        self.single_request::<U64>(
            ETH_CHAIN_ID_METHOD.to_string(),
            Params::Array(vec![]),
            Id::String(ETH_CHAIN_ID_METHOD.to_string()),
        )
        .await
        .map(|v| v.0.to())
    }

    /// Returns balance of the address.
    pub async fn get_balance(&self, address: H160, block: BlockNumber) -> JsonRpcResult<U256> {
        self.single_request(
            ETH_GET_BALANCE_METHOD.to_string(),
            make_params_array!(address, block),
            Id::String(ETH_GET_BALANCE_METHOD.to_string()),
        )
        .await
    }

    /// Returns the gas price
    pub async fn gas_price(&self) -> JsonRpcResult<U256> {
        self.single_request(
            ETH_GAS_PRICE_METHOD.to_string(),
            make_params_array!(),
            Id::String(ETH_GAS_PRICE_METHOD.to_string()),
        )
        .await
    }

    /// Returns the max price per gas
    pub async fn max_priority_fee_per_gas(&self) -> JsonRpcResult<U256> {
        self.single_request(
            ETH_MAX_PRIORITY_FEE_PER_GAS_METHOD.to_string(),
            make_params_array!(),
            Id::String(ETH_MAX_PRIORITY_FEE_PER_GAS_METHOD.to_string()),
        )
        .await
    }

    /// Returns code of the given contract.
    pub async fn get_code(&self, address: H160, block: BlockNumber) -> JsonRpcResult<String> {
        self.single_request(
            ETH_GET_CODE_METHOD.to_string(),
            make_params_array!(address, block),
            Id::String("eth_getCode".to_string()),
        )
        .await
    }

    /// Returns transaction count of the address.
    pub async fn get_transaction_count(
        &self,
        address: H160,
        block: BlockNumber,
    ) -> JsonRpcResult<u64> {
        self.single_request::<U64>(
            ETH_GET_TRANSACTION_COUNT_METHOD.to_string(),
            make_params_array!(address, block),
            Id::String(ETH_GET_TRANSACTION_COUNT_METHOD.to_string()),
        )
        .await
        .map(|v| v.0.to())
    }

    /// Performs eth call and return the result.
    pub async fn eth_call(
        &self,
        params: &TransactionRequest,
        block: BlockNumber,
    ) -> JsonRpcResult<String> {
        self.single_request(
            ETH_CALL_METHOD.to_string(),
            make_params_array!(params, block),
            Id::String("eth_call".to_string()),
        )
        .await
    }

    /// Sends raw transaction and takes the arguments in bytes.
    pub async fn send_raw_transaction_bytes(&self, transaction: &[u8]) -> JsonRpcResult<H256> {
        let transaction = format!("0x{}", alloy::hex::encode(transaction));
        self.single_request(
            ETH_SEND_RAW_TRANSACTION_METHOD.to_string(),
            make_params_array!(transaction),
            Id::String(ETH_SEND_RAW_TRANSACTION_METHOD.to_string()),
        )
        .await
    }

    /// Gets transaction by hash.
    pub async fn get_transaction_by_hash(&self, hash: H256) -> JsonRpcResult<Option<Transaction>> {
        self.single_request(
            ETH_GET_TRANSACTION_BY_HASH_METHOD.to_string(),
            make_params_array!(hash),
            Id::String(ETH_GET_TRANSACTION_BY_HASH_METHOD.to_string()),
        )
        .await
    }

    /// Sends raw transaction and returns transaction hash
    pub async fn send_raw_transaction(&self, transaction: &TxEnvelope) -> JsonRpcResult<H256> {
        use alloy::eips::eip2718::Encodable2718;
        let bytes = transaction.encoded_2718();
        let transaction = format!("0x{}", alloy::hex::encode(bytes));

        self.single_request(
            ETH_SEND_RAW_TRANSACTION_METHOD.to_string(),
            make_params_array!(transaction),
            Id::String(ETH_SEND_RAW_TRANSACTION_METHOD.to_string()),
        )
        .await
    }

    /// Get EVM logs according to the given parameters.
    pub async fn get_logs(&self, params: EthGetLogsParams) -> JsonRpcResult<Vec<Log>> {
        self.single_request(
            ETH_GET_LOGS_METHOD.to_string(),
            make_params_array!(params),
            Id::String(ETH_GET_LOGS_METHOD.to_string()),
        )
        .await
    }

    /// Returns the transaction execution result by hash
    pub async fn get_tx_execution_result_by_hash(
        &self,
        hash: H256,
    ) -> JsonRpcResult<StorableExecutionResult> {
        let id = Id::String(hash.to_string());
        self.single_request::<Option<StorableExecutionResult>>(
            IC_GET_TX_EXECUTION_RESULT_BY_HASH_METHOD.to_string(),
            make_params_array!(hash.clone()),
            id,
        )
        .await?
        .ok_or_else(|| JsonRpcError::TransactionNotFound(hash))
    }

    /// Returns the transaction execution result by hash in batch
    pub async fn get_tx_execution_results_by_hash(
        &self,
        hashes: impl IntoIterator<Item = H256>,
        max_batch_size: usize,
    ) -> JsonRpcResult<Vec<StorableExecutionResult>> {
        let params = hashes
            .into_iter()
            .enumerate()
            .map(|(index, hash)| -> JsonRpcResult<(Params, Id)> {
                Ok((make_params_array!(hash), Id::Number(index as _)))
            })
            .collect::<JsonRpcResult<Vec<_>>>()?;

        Ok(self
            .batch_request::<Option<StorableExecutionResult>>(
                IC_GET_TX_EXECUTION_RESULT_BY_HASH_METHOD,
                params,
                max_batch_size,
            )
            .await?
            .into_iter()
            .flatten()
            .collect())
    }

    /// Returns the genesis accounts
    pub async fn get_genesis_balances(&self) -> JsonRpcResult<Vec<(H160, U256)>> {
        self.single_request(
            IC_GET_GENESIS_BALANCES.to_string(),
            make_params_array!(),
            Id::String(IC_GET_GENESIS_BALANCES.to_string()),
        )
        .await
    }

    /// Returns the EVM global state
    pub async fn get_evm_global_state(&self) -> JsonRpcResult<EvmGlobalState> {
        self.single_request(
            IC_GET_EVM_GLOBAL_STATE.to_string(),
            make_params_array!(),
            Id::String(IC_GET_EVM_GLOBAL_STATE.to_string()),
        )
        .await
    }

    /// Returns the blockchain block info
    pub async fn get_blockchain_block_info(&self) -> JsonRpcResult<BlockchainBlockInfo> {
        self.single_request(
            IC_GET_BLOCKCHAIN_BLOCK_INFO.to_string(),
            make_params_array!(),
            Id::String(IC_GET_BLOCKCHAIN_BLOCK_INFO.to_string()),
        )
        .await
    }

    /// Returns the last certified block
    pub async fn get_last_certified_block(&self) -> JsonRpcResult<CertifiedResult<Block<H256>>> {
        self.single_request(
            IC_GET_LAST_CERTIFIED_BLOCK.to_string(),
            make_params_array!(),
            Id::String(IC_GET_LAST_CERTIFIED_BLOCK.to_string()),
        )
        .await
    }

    /// Sends the confirm block
    pub async fn send_confirm_block(
        &self,
        params: BlockConfirmationData,
    ) -> JsonRpcResult<BlockConfirmationResult> {
        self.single_request(
            IC_SEND_CONFIRM_BLOCK.to_string(),
            make_params_array!(params),
            Id::String(IC_SEND_CONFIRM_BLOCK.to_string()),
        )
        .await
    }

    /// Performs a request.
    pub async fn request(&self, request: RpcRequest) -> JsonRpcResult<RpcResponse> {
        self.client.send_rpc_request(request).await
    }

    /// Performs a single request.
    pub async fn single_request<R: DeserializeOwned>(
        &self,
        method: String,
        params: Params,
        id: Id,
    ) -> JsonRpcResult<R> {
        let request = RpcRequest::Single(Request {
            params,
            method,
            id,
            ..Default::default()
        });

        let response = self.client.send_rpc_request(request).await?;

        match response {
            RpcResponse::Single(response) => match response {
                Response::Success(success) => {
                    serde_json::from_value(success.result).map_err(JsonRpcError::from)
                }
                Response::Failure(error_payload) => Err(error_payload.into()),
            },
            RpcResponse::Batch(_) => Err(JsonRpcError::UnexpectedBatch),
        }
    }

    /// Performs a batch request.
    pub async fn batch_request<R: DeserializeOwned>(
        &self,
        method: &str,
        params: impl IntoIterator<Item = (Params, Id)>,
        max_batch_size: usize,
    ) -> JsonRpcResult<Vec<R>> {
        let value_from_json =
            |value| serde_json::from_value::<R>(value).map_err(JsonRpcError::from);

        let raw_results = self
            .batch_request_raw(
                params.into_iter().map(|(param, id)| (method, param, id)),
                max_batch_size,
            )
            .await?;

        raw_results
            .into_iter()
            .map(value_from_json)
            .collect::<JsonRpcResult<Vec<R>>>()
    }

    /// Performs a batch request to different eth methods.
    pub async fn batch_request_raw(
        &self,
        params: impl IntoIterator<Item = (&str, Params, Id)>,
        max_batch_size: usize,
    ) -> JsonRpcResult<Vec<Value>> {
        let mut results = Vec::new();

        // Collect chunks before iteration, otherwise the future won't be `Send`
        let chunks: Vec<Vec<(&str, Params, Id)>> = params
            .into_iter()
            .chunks(max_batch_size)
            .into_iter()
            .map(Iterator::collect::<Vec<_>>)
            .collect::<Vec<_>>();
        for chunk in chunks {
            let requests = chunk
                .into_iter()
                .map(|(method, params, id)| Request {
                    // meta: RequestMeta::new(method.to_string().into(), id),
                    method: method.to_string(),
                    id,
                    params,
                    jsonrpc: Some(Version::V2),
                })
                .collect::<Vec<_>>();
            let chunk_size = requests.len();
            let request = RpcRequest::Batch(requests);

            let response = self.client.send_rpc_request(request).await?;

            match response {
                RpcResponse::Single(response) => match response {
                    Response::Success(result) => {
                        if chunk_size == 1 {
                            results.push(result.result);
                        } else {
                            return Err(JsonRpcError::UnexpectedResultsAmount {
                                expected: chunk_size,
                                actual: 1,
                            });
                        }
                    }
                    Response::Failure(err) => {
                        return Err(err.into());
                    }
                },
                RpcResponse::Batch(response) => {
                    if chunk_size == response.len() {
                        for resp in response.into_iter() {
                            match resp {
                                Response::Success(resp) => {
                                    results.push(resp.result);
                                }
                                Response::Failure(err) => {
                                    return Err(err.into());
                                }
                            }
                        }
                    } else {
                        Err(JsonRpcError::UnexpectedResultsAmount {
                            expected: chunk_size,
                            actual: response.len(),
                        })?;
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
        request: RpcRequest,
    ) -> Pin<Box<dyn Future<Output = JsonRpcResult<RpcResponse>> + Send>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eth_get_logs_params_serialization() {
        let get_logs_params = EthGetLogsParams {
            address: Some(vec![
                H160::from_hex_str("0xb59f67a8bff5d8cd03f6ac17265c550ed8f33907").unwrap(),
            ]),
            from_block: BlockNumber::Number(42u64.into()),
            to_block: BlockNumber::Latest,
            topics: Some(vec![
                vec![
                    H256::from_hex_str(
                        "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef",
                    )
                    .unwrap(),
                ],
                vec![
                    H256::from_hex_str(
                        "0x00000000000000000000000000b46c2526e227482e2ebb8f4c69e4674d262e75",
                    )
                    .unwrap(),
                ],
                vec![
                    H256::from_hex_str(
                        "0x00000000000000000000000054a2d42a40f51259dedd1978f6c118a0f0eff078",
                    )
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
