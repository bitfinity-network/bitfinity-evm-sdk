use did::error::EvmError;

/// This is the result type for all EVM calls.
pub type EvmResult<T> = Result<T, EvmError>;
