use std::borrow::Cow;
use std::mem::size_of;
use std::time::Duration;

use candid::{CandidType, Principal};
use did::codec::ByteChunkReader;
use did::{H160, U256};
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

impl OperationPricing {
    pub const STORABLE_BYTE_SIZE: usize = size_of::<u32>() * 6;
}

impl Storable for OperationPricing {
    fn to_bytes(&self) -> Cow<'_, [u8]> {
        let mut buf = Vec::with_capacity(Self::STORABLE_BYTE_SIZE);
        buf.extend_from_slice(&self.evmc_notification.to_be_bytes());
        buf.extend_from_slice(&self.evm_registration.to_be_bytes());
        buf.extend_from_slice(&self.icrc_mint_approval.to_be_bytes());
        buf.extend_from_slice(&self.icrc_transfer.to_be_bytes());
        buf.extend_from_slice(&self.erc20_mint.to_be_bytes());
        buf.extend_from_slice(&self.endpoint_query.to_be_bytes());
        buf.into()
    }

    fn from_bytes(bytes: Cow<'_, [u8]>) -> Self {
        let mut reader = ByteChunkReader::new(&bytes);
        let evmc_notification = u32::from_be_bytes(*reader.read_slice());
        let evm_registration = u32::from_be_bytes(*reader.read_slice());
        let icrc_mint_approval = u32::from_be_bytes(*reader.read_slice());
        let icrc_transfer = u32::from_be_bytes(*reader.read_slice());
        let erc20_mint = u32::from_be_bytes(*reader.read_slice());
        let endpoint_query = u32::from_be_bytes(*reader.read_slice());
        Self {
            evmc_notification,
            evm_registration,
            icrc_mint_approval,
            icrc_transfer,
            erc20_mint,
            endpoint_query,
        }
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: Self::STORABLE_BYTE_SIZE as _,
        is_fixed_size: true,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operation_pricing_storable_roundtrip() {
        let operation_pricing = OperationPricing {
            evmc_notification: rand::random(),
            evm_registration: rand::random(),
            icrc_mint_approval: rand::random(),
            icrc_transfer: rand::random(),
            erc20_mint: rand::random(),
            endpoint_query: rand::random(),
        };

        let bytes = operation_pricing.to_bytes();
        let decoded = OperationPricing::from_bytes(bytes);

        assert_eq!(operation_pricing, decoded);
    }
}
