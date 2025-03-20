use std::time::Duration;

use candid::{CandidType, Nat, Principal};
use ic_log::LogSettings;
use serde::{Deserialize, Serialize};

use crate::permission::Permission;
use crate::{H160, U256};

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
    #[serde(default)]
    pub reserve_memory_pages: Option<u64>,
    /// Owner of the EVM Canister
    pub owner: Principal,
    /// Genesis accounts
    pub genesis_accounts: Vec<(H160, Option<U256>)>,
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
            reserve_memory_pages: Default::default(),
            transaction_processing_interval: Default::default(),
            owner: Principal::management_canister(),
            genesis_accounts: vec![],
            coinbase: Default::default(),
        }
    }
}

/// These are the arguments which are taken by the signature verification canister init fn
#[derive(Debug, Clone, Serialize, CandidType, Deserialize)]
pub struct SignatureVerificationCanisterInitData {
    /// Access list of principals that are allowed to send transactions to the EVM canisters
    pub access_list: Vec<Principal>,
    /// EVM canister Principal
    pub evm_canister: Principal,
    /// Interval for pushing transactions to the EVM canisters
    pub pushing_timer_interval: Duration,
}
