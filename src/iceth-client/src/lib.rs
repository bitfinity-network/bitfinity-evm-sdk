use candid::{CandidType, Deserialize, Principal};
use did::{BlockNumber, Bytes, TransactionReceipt, H160, H256, U256};
use ethers_core::types::Transaction as EthTransaction;
use ic_canister::virtual_canister_call;
use ic_exports::ic_kit::RejectionCode;
use jsonrpc_core::Output;
use thiserror::Error;

#[derive(Debug, Clone, Error, Deserialize, CandidType, Eq, PartialEq)]
pub enum Error {
    #[error("inter-canister call failed with code {0:?}: {1}")]
    CallFailed(RejectionCode, String),

    #[error(transparent)]
    IcethError(#[from] IcethError),

    #[error("serialization error: {0}")]
    SerializationError(String),

    #[error("JSON-RPC call failed: {0:?}")]
    JsonRpcFailure(String),
}

impl From<(RejectionCode, String)> for Error {
    fn from(error: (RejectionCode, String)) -> Self {
        Self::CallFailed(error.0, error.1)
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Self::SerializationError(error.to_string())
    }
}

impl From<jsonrpc_core::Failure> for Error {
    fn from(failure: jsonrpc_core::Failure) -> Self {
        Self::JsonRpcFailure(failure.error.to_string())
    }
}

#[derive(Debug, Clone, Error, Deserialize, CandidType, Eq, PartialEq)]
pub enum IcethError {
    #[error("no permission")]
    NoPermission,

    #[error("too few cycles: {0}")]
    TooFewCycles(String),

    #[error("service url parse error")]
    ServiceUrlParseError,

    #[error("service url host missing")]
    ServiceUrlHostMissing,

    #[error("service url host not allowed")]
    ServiceUrlHostNotAllowed,

    #[error("provider not found")]
    ProviderNotFound,

    #[error("http request error {code}: {message}")]
    HttpRequestError { code: u32, message: String },
}

pub struct Client {
    iceth_principal: Principal,
    url: String,
}

impl Client {
    /// Creates a new client instance.
    pub fn new(iceth_principal: Principal, url: String) -> Self {
        Self {
            iceth_principal,
            url,
        }
    }

    /// Returns information about the external evm.
    pub fn get_url(&self) -> &str {
        &self.url
    }

    /// Returns balance of the given address.
    pub async fn get_balance(&self, address: &H160, block: BlockNumber) -> Result<U256, Error> {
        let data = format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"eth_getBalance","params":["{address:#x}", "{block}"]}}"#,
        );
        let result = self.json_rpc_call(&data).await?;
        self.process_json_rpc_response(&result)
    }

    /// Returns minimal gas price.
    pub async fn gas_price(&self) -> Result<U256, Error> {
        let data = r#"{"jsonrpc":"2.0","id":1,"method":"eth_gasPrice","params":[]}"#;
        let result = self.json_rpc_call(data).await?;
        self.process_json_rpc_response(&result)
    }

    /// Returns transactions count for the given address.
    pub async fn get_transaction_count(
        &self,
        address: &H160,
        block: BlockNumber,
    ) -> Result<U256, Error> {
        let data = format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"eth_getTransactionCount","params":["{address:#x}", "{block}"]}}"#,
        );
        let result = self.json_rpc_call(&data).await?;
        self.process_json_rpc_response(&result)
    }

    /// Sends a raw transaction to the external EVM.
    pub async fn send_raw_transaction(&self, transaction_bytes: Bytes) -> Result<H256, Error> {
        let data = format!(
            r#"{{"jsonrpc":"2.0","id":"3","method":"eth_sendRawTransaction","params":["0x{transaction_bytes:x}"]}}"#,
        );
        let result = self.json_rpc_call(&data).await?;
        self.process_json_rpc_response(&result)
    }

    /// Returns transaction by hash.
    pub async fn get_transaction_by_hash(
        &self,
        tx_hash: H256,
    ) -> Result<Option<EthTransaction>, Error> {
        let data = format!(
            r#"{{"jsonrpc":"2.0","id":"3","method":"eth_getTransactionByHash","params":["{tx_hash:#x}"]}}"#,
        );
        let result = self.json_rpc_call(&data).await?;
        self.process_json_rpc_response(&result)
    }

    /// Returns transaction receipt.
    pub async fn get_transaction_receipt(
        &self,
        tx_hash: &H256,
    ) -> Result<Option<TransactionReceipt>, Error> {
        let data = format!(
            r#"{{"jsonrpc":"2.0","id":"3","method":"eth_getTransactionReceipt","params":["{tx_hash:#x}"]}}"#,
        );
        let result = self.json_rpc_call(&data).await?;
        self.process_json_rpc_response(&result)
    }

    async fn json_rpc_call(&self, data: &str) -> Result<Vec<u8>, Error> {
        Ok(virtual_canister_call!(
            self.iceth_principal,
            "json_rpc_request",
            (data, &self.url, 2048_u64),
            Result<Vec<u8>, IcethError>,
            628644000000 // TODO: calculate
        )
        .await??)
    }

    fn process_json_rpc_response<T>(&self, data: &[u8]) -> Result<T, Error>
    where
        T: for<'a> Deserialize<'a>,
    {
        let output = serde_json::from_slice::<Output>(data)?;

        let result = match output {
            Output::Success(success) => success.result,
            Output::Failure(failure) => return Err(failure.into()),
        };

        Ok(serde_json::from_value::<T>(result)?)
    }
}

/// Methods available only for bitfinity EVM implementation.
#[cfg(feature = "bitfinity")]
impl Client {
    /// Mints BETH tokens for the given address. Available only in testnet.
    pub async fn mint_evm_token(&self, address: &H160, amount: U256) -> Result<U256, Error> {
        let data = format!(
            r#"{{"jsonrpc":"2.0","id":"3","method":"ic_mintEVMToken","params":["{address:#x}", "{amount:#x}"]}}"#,
        );
        let result = self.json_rpc_call(&data).await?;
        self.process_json_rpc_response(&result)
    }
}
