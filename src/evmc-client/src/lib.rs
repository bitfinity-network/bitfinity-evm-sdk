#[cfg(feature = "ic-agent-client")]
pub mod agent;

pub mod client;
pub mod ic_client;

pub use client::EvmcClient;
use did::error::EvmError;
use ic_exports::ic_kit::RejectionCode;

/// This tuple is returned incase of IC errors such as Network, canister error.
pub type IcError = (RejectionCode, String);

/// This is the result type for all IC calls.
pub type IcResult<R> = Result<R, IcError>;

/// This is the result type for all EVM calls.
pub type EvmResult<T> = Result<T, EvmError>;
