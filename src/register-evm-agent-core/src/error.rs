use did::error::EvmError;
use did::H256;
use eth_signer::WalletError;
use evm_canister_client::CanisterClientError;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[cfg(feature = "ic-agent-client")]
    #[error("IPC agent error: {0}")]
    Agent(evm_canister_client::ic_agent::AgentError),
    #[error("address is already reserved")]
    AlreadyReserved,
    #[error("EVM error: {0}")]
    Evm(EvmError),
    #[error("canister client error: {0}")]
    CanisterClient(CanisterClientError),
    #[error("parse error: {0}")]
    Parse(candid::Error),
    #[error("wallet error: {0}")]
    Wallet(WalletError),

    #[error("transaction not finalized {0}")]
    TransactionNotFinalized(H256),
    #[error("transaction failed")]
    TransactionFailed,
}

#[cfg(feature = "ic-agent-client")]
impl From<evm_canister_client::ic_agent::AgentError> for Error {
    fn from(err: evm_canister_client::ic_agent::AgentError) -> Self {
        Self::Agent(err)
    }
}

impl From<candid::Error> for Error {
    fn from(err: candid::Error) -> Self {
        Self::Parse(err)
    }
}

impl From<WalletError> for Error {
    fn from(err: WalletError) -> Self {
        Self::Wallet(err)
    }
}

impl From<EvmError> for Error {
    fn from(err: EvmError) -> Self {
        Self::Evm(err)
    }
}

impl From<CanisterClientError> for Error {
    fn from(err: CanisterClientError) -> Self {
        Self::CanisterClient(err)
    }
}
