use std::borrow::Cow;

use candid::{CandidType, Deserialize};
use jsonrpc_core::{Error, ErrorCode};
use rlp::DecoderError;
use serde::Serialize;
use thiserror::Error;

use crate::{BlockNumber, U256, H160};

pub type Result<T> = std::result::Result<T, EvmError>;

#[derive(Debug, Error, Deserialize, CandidType, Eq, PartialEq)]
pub enum EvmError {
    #[error("internal error: {0}")]
    Internal(String),

    #[error("insufficient balance: actual: {actual}, expected {expected}")]
    InsufficientBalance {
        actual: crate::U256,
        expected: crate::U256,
    },

    #[error("evm transaction failed due to {0:?}")]
    NotProcessableTransactionError(HaltError),

    #[error("evm transaction failed due to {0:?}")]
    FatalEvmExecutorError(ExitFatal),

    #[error("gas price should be >= {0}")]
    InvalidGasPrice(crate::U256),

    #[error("the user has no permission to call this method")]
    NotAuthorized,

    #[error("reservation failed: {0}")]
    ReservationFailed(String),

    #[error("Stable Storage error: {0}")]
    StableStorageError(String),

    #[error("transaction pool error {0}")]
    TransactionPool(TransactionPoolError),

    #[error("no history state data for block {0}")]
    NoHistoryDataForBlock(BlockNumber),

    #[error("block doesn't exist: {0}")]
    BlockDoesNotExist(BlockNumber),

    #[error("Transaction Signature error: {0}")]
    TransactionSignature(String),

    #[error("gas is too low, minimum required: {minimum}")]
    GasTooLow { minimum: U256 },

    #[error("anonymous caller is not allowed")]
    AnonymousPrincipal,
}

/// Variant of `TransactionPool` error
#[derive(Debug, Deserialize, Error, CandidType, PartialEq, Eq)]
pub enum TransactionPoolError {
    #[error("transaction already exists in the pool")]
    TransactionAlreadyExists,

    #[error("invalid transaction nonce, expected {expected}, actual {actual}")]
    InvalidNonce { expected: U256, actual: U256 },

    #[error("the maximum amount of transactions per sender has been reached")]
    TooManyTransactions,

    #[error("transaction gas price is too low to replace an existing transaction")]
    TxReplacementUnderpriced,
}

impl EvmError {
    pub fn unsupported_method_error() -> Self {
        Self::Internal("method is not supported".to_string())
    }
}

impl From<String> for EvmError {
    fn from(msg: String) -> Self {
        Self::Internal(msg)
    }
}

impl From<DecoderError> for EvmError {
    fn from(decode_error: DecoderError) -> Self {
        Self::Internal(format!("rlp err: {decode_error}"))
    }
}

impl From<serde_json::Error> for EvmError {
    fn from(err: serde_json::Error) -> Self {
        Self::Internal(format!("JSON encoding error: {err}"))
    }
}

/// https://docs.alchemy.com/reference/error-reference#kovan-error-codes
impl From<EvmError> for jsonrpc_core::error::Error {
    fn from(err: EvmError) -> Self {
        let code = match &err {
            EvmError::InsufficientBalance {
                actual: _,
                expected: _,
            } => -32010, // TRANSACTION_ERROR
            EvmError::InvalidGasPrice(_) => -32016, // ACCOUNT_ERROR
            EvmError::NotAuthorized => -32002,      // NO_AUTHOR
            _ => -32015,                            // EXECUTION_ERROR
        };
        Error {
            code: ErrorCode::ServerError(code),
            message: err.to_string(),
            data: None,
        }
    }
}

#[derive(
    Debug, Clone, Serialize, Deserialize, CandidType, Eq, PartialEq, PartialOrd, Ord, Hash,
)]
pub enum HaltError {
    /// Trying to pop from an empty stack.
    StackUnderflow,
    /// Trying to push into a stack over stack limit.
    StackOverflow,
    /// Jump destination is invalid.
    InvalidJump,
    /// An opcode accesses memory region, but the region is invalid.
    InvalidRange,
    /// Encountered the designated invalid opcode.
    DesignatedInvalid,
    /// Call stack is too deep (runtime).
    CallTooDeep,
    /// Create opcode encountered collision (runtime).
    CreateCollision,
    /// Create init code exceeds limit (runtime).
    CreateContractLimit,
    /// Starting byte must not begin with 0xef. See [EIP-3541](https://github.com/ethereum/EIPs/blob/master/EIPS/eip-3541.md).
    InvalidCode(u8),

    /// An opcode accesses external information, but the request is off offset
    /// limit (runtime).
    OutOfOffset,
    /// Execution runs out of gas (runtime).
    OutOfGas,
    /// Not enough fund to start the execution (runtime).
    OutOfFund,

    /// PC underflowed (unused).
    PCUnderflow,

    /// Attempt to create an empty account (runtime, unused).
    CreateEmpty,

    /// Other normal errors.
    Other(Cow<'static, str>),
    OpcodeNotFound,
    CallNotAllowedInsideStatic,
    InvalidOpcode,
    NotActivated,
    FatalExternalError,
    GasMaxFeeGreaterThanPriorityFee,
    GasPriceLessThanBasefee,
    CallerGasLimitMoreThanBlock,
    RejectCallerWithCode,
    LackOfFundForGasLimit,
    OverflowPayment,
    PrecompileError,
    NonceOverflow,
    CreateContractWithEF,
    PrevrandaoNotSet,
    Continue,
    Revert(Option<String>),
    CallGasCostMoreThanGasLimit,
    NonceTooHigh,
    NonceTooLow,
    CreateInitcodeSizeLimit,
    InvalidChainId,
    StateChangeDuringStaticCall,
}

#[derive(Debug, Clone, Deserialize, CandidType, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub enum ExitFatal {
    /// The operation is not supported.
    NotSupported,
    /// The trap (interrupt) is unhandled.
    UnhandledInterrupt,
    /// The environment explicitly set call errors as fatal error.
    CallErrorAsFatal(HaltError),

    /// Other fatal errors.
    Other(Cow<'static, str>),
}

impl From<HaltError> for EvmError {
    fn from(exit_err: HaltError) -> Self {
        Self::NotProcessableTransactionError(exit_err)
    }
}

#[derive(Error, Debug, CandidType, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum SignatureVerificationError {
    #[error("signature is not correct: expected: {expected}, recovered: {recovered}")]
    RecoveryError { expected: H160, recovered: H160 },
    #[error("failed to verify signature: {0}")]
    InternalError(String),
}
