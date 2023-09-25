use std::time::Duration;

use candid::{CandidType, Principal};
use did::sign_strategy::SigningStrategy;
use did::{H160, U256};
use ic_log::LogSettings;
use serde::Deserialize;

/// Minter canister initialization data.
#[derive(Debug, Deserialize, CandidType, Clone)]
pub struct InitData {
    /// Principal of canister's owner.
    pub owner: Principal,

    /// Principal of EVM canister, in which minter canister will withdraw/deposit tokens.
    pub evm_principal: Principal,

    /// Principal of ICETH canister, which is used for JSON-RPC outcalls.
    pub iceth_principal: Principal,

    /// EVMC chain id
    pub evm_chain_id: u32,

    /// BFT bridge contract address, if it exists already.
    pub bft_bridge_contract: Option<H160>,

    /// Gas price for evm transactions
    pub evm_gas_price: U256,

    /// Principal of spender canister, which is used for secure token transfers.
    pub spender_principal: Principal,

    /// Signing strategy
    pub signing_strategy: SigningStrategy,

    /// Process transactions results interval.
    #[serde(default)]
    pub process_transactions_results_interval: Option<Duration>,

    /// Log settings
    #[serde(default)]
    pub log_settings: Option<LogSettings>,
}
