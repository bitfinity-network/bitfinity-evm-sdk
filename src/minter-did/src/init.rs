use candid::{CandidType, Principal};
use did::H160;
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

    /// Signing strategy
    pub signing_strategy: SigningStrategy,

    /// Address of a contract, which implements IFeeCharge.
    /// If not set, fee will not be charged.
    pub fee_charge_contract: Option<H160>,

    /// Log settings
    #[serde(default)]
    pub log_settings: Option<LogSettings>,
}
