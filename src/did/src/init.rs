use std::time::Duration;

use candid::{CandidType, Nat, Principal};
use ic_log::LogSettings;
use serde::Deserialize;

use crate::{H160, U256};

pub type GenesisAccount = (H160, Option<U256>);

/// These are the arguments which are taken by the evm canister init fn
#[derive(Debug, Clone, CandidType, Deserialize)]
pub struct EvmCanisterInitData {
    pub signature_verification_principal: Principal,
    pub min_gas_price: Nat,
    pub chain_id: u64,
    #[serde(default)]
    pub log_settings: Option<LogSettings>,
    #[serde(default)]
    pub permissions: Option<Vec<(Principal, Vec<Permission>)>>,
    #[serde(default)]
    pub transaction_processing_interval: Option<Duration>,
    /// Owner of the EVM Canister
    pub owner: Principal,
    /// Genesis accounts
    pub genesis_accounts: Vec<GenesisAccount>,
    /// Coinbase address
    pub coinbase: H160,
}

impl Default for EvmCanisterInitData {
    fn default() -> Self {
        Self {
            signature_verification_principal: Principal::anonymous(),
            min_gas_price: Default::default(),
            chain_id: Default::default(),
            log_settings: Default::default(),
            permissions: Default::default(),
            transaction_processing_interval: Default::default(),
            owner: Principal::management_canister(),
            genesis_accounts: vec![],
            coinbase: Default::default(),
        }
    }
}

/// Principal specific permission
#[derive(
    Debug, Clone, CandidType, Deserialize, Hash, PartialEq, Eq, PartialOrd, Ord, serde::Serialize,
)]
pub enum Permission {
    /// Gives administrator permissions
    Admin,
    /// Allows calling the endpoints to read the logs and get runtime statistics
    ReadLogs,
    /// Allows calling the endpoints to set the logs configuration
    UpdateLogsConfiguration,
    /// Allows caller to update blockchain history
    UpdateBlockchain,
}
