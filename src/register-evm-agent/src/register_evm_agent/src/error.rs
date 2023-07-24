use std::path::PathBuf;

use candid::Principal;
use did::error::EvmError;
use did::H256;
use eth_signer::WalletError;
use evm_canister_client::CanisterClientError;
use evm_canister_client::ic_agent::identity::PemError;
use evm_canister_client::ic_agent::AgentError;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IPC agent error: {0}")]
    Agent(AgentError),
    #[error("address is already reserved: {0}")]
    AlreadyReserved(Principal),
    #[error("failed to get agent principal: {0}")]
    CouldNotGetPrincipal(String),
    #[error("EVM error: {0}")]
    Evm(EvmError),
    #[error("canister client error: {0}")]
    CanisterClientError(CanisterClientError),
    #[error("parse error: {0}")]
    Parse(candid::Error),
    #[error("failed to read PEM file {0}: {1}")]
    Pem(PathBuf, PemError),
    #[error("wallet error: {0}")]
    Wallet(WalletError),

    #[error("transaction not finalized {0}")]
    TransactionNotFinalized(H256),
    #[error("transaction failed")]
    TransactionFailed,
}

impl From<AgentError> for Error {
    fn from(err: AgentError) -> Self {
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
        Self::CanisterClientError(err)
    }
}
