use std::borrow::Cow;
use std::time::Duration;

use candid::{CandidType, Principal};
use did::{codec, H160, U256};
use eth_signer::sign_strategy::SigningStrategy;
use ic_log::LogSettings;
use ic_stable_structures::{Bound, Storable};
use serde::Deserialize;

/// Minter canister initialization data.
#[derive(Debug, Deserialize, CandidType, Clone)]
pub struct InitData {
    /// Principal of canister's owner.
    pub owner: Principal,

    /// Principal of EVM canister, in which minter canister will withdraw/deposit tokens.
    pub evm_principal: Principal,

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

#[derive(Debug, PartialEq, Eq, Copy, Clone, CandidType, Deserialize, serde::Serialize)]
pub struct OperationPricing {
    pub evmc_notification: u32,
    pub evm_registration: u32,
    pub icrc_mint_approval: u32,
    pub icrc_transfer: u32,
    pub erc20_mint: u32,
    pub endpoint_query: u32,
}

impl Default for OperationPricing {
    fn default() -> Self {
        Self {
            evmc_notification: 8,
            evm_registration: 1,
            icrc_mint_approval: 1,
            icrc_transfer: 1,
            erc20_mint: 1,
            endpoint_query: 1,
        }
    }
}

// impl Storable for OperationPricing {
//     fn to_bytes(&self) -> Cow<'_, [u8]> {
//         codec::encode(self).into()
//     }

//     fn from_bytes(bytes: Cow<'_, [u8]>) -> Self {
//         codec::decode(&bytes)
//     }

//     const BOUND: ic_stable_structures::Bound;
// }
