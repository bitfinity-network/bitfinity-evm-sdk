use candid::{CandidType, Deserialize};
use did::H160;
use ic_canister_client::CanisterClientError;
use ic_exports::ic_kit::RejectionCode;
use ic_exports::icrc_types::icrc1::transfer::TransferError;
use ic_exports::icrc_types::icrc2::transfer_from::TransferFromError;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

/// Minter canister operation error.
#[derive(Debug, Error, Deserialize, CandidType, PartialEq, Eq, Clone)]
pub enum Error {
    #[error("internal error: {0}")]
    Internal(String),

    #[error("the user has no permission to call this method")]
    NotAuthorized,

    #[error("icrc2 transfer failed: {0:?}")]
    Icrc2TransferError(TransferError),

    #[error("inter-canister call failed with code {0:?}: {1}")]
    InterCanisterCallFailed(RejectionCode, String),

    #[error("icrc2 approval failed: {0:?}")]
    Icrc2TransferFromError(TransferFromError),

    #[error("BftBridge contract doesn't exist")]
    BftBridgeDoesNotExist,

    #[error("BftBridge contract is invalid")]
    InvalidBftBridgeContract,

    #[error("Invalid token address")]
    InvalidTokenAddress,

    #[error("JSON-RPC method error")]
    JsonRpcCallFailed(String),

    #[error("anonymous principal is not allowed")]
    AnonymousPrincipal,

    #[error("invalid deposit transaction: {0}")]
    InvalidBurnOperation(String),

    #[error("EVM with chain ID '{0}' already registered")]
    BftBridgeAlreadyRegistered(H160),

    #[error("expected nonce >= {minimum}, got {got}")]
    InvalidNonce { minimum: u64, got: u64 },

    #[error("not enough operation points: expected {expected}, got {got}")]
    InsufficientOperationPoints { expected: u32, got: u32 },
}

impl From<String> for Error {
    fn from(value: String) -> Self {
        Self::Internal(value)
    }
}

impl From<(RejectionCode, String)> for Error {
    fn from(value: (RejectionCode, String)) -> Self {
        Self::InterCanisterCallFailed(value.0, value.1)
    }
}

impl From<TransferFromError> for Error {
    fn from(value: TransferFromError) -> Self {
        Self::Icrc2TransferFromError(value)
    }
}

impl From<TransferError> for Error {
    fn from(value: TransferError) -> Self {
        Self::Icrc2TransferError(value)
    }
}

impl From<CanisterClientError> for Error {
    fn from(value: CanisterClientError) -> Self {
        Self::Internal(value.to_string())
    }
}
