use candid::{CandidType, Principal};
use eth_signer::sign_strategy::SigningStrategy;
use ic_log::LogSettings;
use serde::Deserialize;

/// Minter canister initialization data.
#[derive(Debug, Deserialize, CandidType, Clone)]
pub struct InitData {
    /// Principal of canister's owner.
    pub owner: Principal,

    /// Principal of EVM canister, in which minter canister will withdraw/deposit tokens.
    pub evm_principal: Principal,

    /// Principal of spender canister, which is used for secure token transfers.
    pub spender_principal: Principal,

    /// Signing strategy
    pub signing_strategy: SigningStrategy,

    /// Log settings
    #[serde(default)]
    pub log_settings: Option<LogSettings>,
}
