use std::path::PathBuf;

use candid::Principal;
use did::error::EvmError;
use eth_signer::WalletError;
use ic_agent::identity::PemError;
use ic_agent::AgentError;
use rlp::DecoderError;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IPC agent error: {0}")]
    Agent(AgentError),
    #[error("wallet is already registered: {0}")]
    AlreadyRegistered(Principal),
    #[error("Failed to check registration status:\n  Wallet Address = {0}\n  Principal = {1}")]
    CouldNotCheckRegistrationStatus(String, Principal),
    #[error("failed to get agent principal: {0}")]
    CouldNotGetPrincipal(String),
    #[error("failed to get registration info: {0}")]
    Decoder(DecoderError),
    #[error("EVM error: {0}")]
    Evm(EvmError),
    #[error("parse error: {0}")]
    Parse(candid::Error),
    #[error("failed to read PEM file {0}: {1}")]
    Pem(PathBuf, PemError),
    #[error("wallet error: {0}")]
    Wallet(WalletError),
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

impl From<DecoderError> for Error {
    fn from(err: DecoderError) -> Self {
        Self::Decoder(err)
    }
}

impl From<EvmError> for Error {
    fn from(err: EvmError) -> Self {
        Self::Evm(err)
    }
}
