//! Error types for the Ethereum JSON-RPC client.

use did::H256;
use did::rpc::response::Failure;
use ic_exports::ic_cdk::call::Error as CallError;
use thiserror::Error;

/// Result type for the Ethereum JSON-RPC client.
pub type JsonRpcResult<T> = std::result::Result<T, JsonRpcError>;

/// Error type for the Ethereum JSON-RPC client.
#[derive(Error, Debug)]
pub enum JsonRpcError {
    /// Canister client error [`ic_canister_client::CanisterClientError`]
    #[cfg(feature = "ic-canister-client")]
    #[error("Canister client error: {0}")]
    CanisterClient(#[from] ic_canister_client::CanisterClientError),
    #[error("Canister call failed: {0}")]
    CanisterCall(#[from] CallError),
    /// Error while parsing the JSON response.
    #[error("Invalid JSON response: {0}")]
    Json(#[from] serde_json::Error),
    /// EVM failed to process the request. See [`Failure`] for details,
    /// and [`did::rpc::error::Error`] in particular to get the message and the code.
    #[error("EVM error: {0}")]
    Evm(Failure),
    /// HTTP error.
    #[cfg(feature = "reqwest")]
    #[error("HTTP error {code}: {text}")]
    Http {
        /// HTTP status code.
        code: reqwest::StatusCode,
        /// HTTP response text.
        text: String,
    },
    /// There were not enough cycles to send the request.
    #[error("Insufficient cycles: available {available}, required {cost}")]
    InsufficientCycles {
        /// The amount of cycles that are available.
        available: u128,
        /// The amount of cycles that are required.
        cost: u128,
    },
    /// Reqwest error.
    #[cfg(feature = "reqwest")]
    #[error("Reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),
    /// Fetched transaction information for a transaction that hasn't been found.
    #[error("transaction {0} not found")]
    TransactionNotFound(H256),
    /// A single request was sent, but a batch response was received.
    #[error("unexpected batch response: expected single but got batch")]
    UnexpectedBatch,
    /// A batch request was sent, but the number of responses is not equal to the number of requests.
    #[error("unexpected response: expected {expected} but got {actual}")]
    UnexpectedResultsAmount { expected: usize, actual: usize },
    /// The URL provided is invalid, because it is missing the host name.
    #[cfg(feature = "http-outcall")]
    #[error("provided host is missing the host name: {0}")]
    UrlMissingHost(url::Url),
    /// Error while parsing the URL.
    #[cfg(feature = "http-outcall")]
    #[error("Invalid URL: {0}")]
    UrlParser(#[from] url::ParseError),
}

impl From<Failure> for JsonRpcError {
    fn from(err: Failure) -> Self {
        JsonRpcError::Evm(err)
    }
}
