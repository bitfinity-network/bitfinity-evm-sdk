use std::borrow::Cow;

use candid::Principal;
use config::Config;
use ic_exports::stable_structures::memory_manager::MemoryId;
use ic_stable_structures::{BoundedStorable, Storable};
pub use pair_price::PairPrice;

mod config;
pub mod http;
mod pair_price;

pub const CONFIG_MEMORY_ID: MemoryId = MemoryId::new(80);
pub const PRICE_MEMORY_ID: MemoryId = MemoryId::new(81);
pub const LATEST_TIME_MEMORY_ID: MemoryId = MemoryId::new(82);
pub const PAIR_MEMORY_ID: MemoryId = MemoryId::new(83);
pub const PRICE_MULTIPLE: f64 = 1_0000_0000.0;

/// State of a minter canister.
#[derive(Default)]
pub struct State {
    /// Minter canister configuration.
    pub config: Config,

    /// Set of token pairs like (ic_token_principal, evm_token_contract_address);
    pub pair_price: PairPrice,
}

impl State {
    /// Clear the state and set initial data from settings.
    pub fn reset(&mut self, settings: Settings) {
        self.config.reset(settings);
        self.pair_price.reset();
    }
}

/// State settings.
#[derive(Debug, Clone, Copy)]
pub struct Settings {
    pub owner: Principal,
    pub evmc_principal: Principal,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            owner: Principal::anonymous(),
            evmc_principal: Principal::anonymous(),
        }
    }
}

/// Storable String. used as a stable storage pair name.
#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct PairKey(pub String);

impl Storable for PairKey {
    fn to_bytes(&self) -> Cow<'_, [u8]> {
        self.0.to_bytes()
    }

    fn from_bytes(bytes: Cow<'_, [u8]>) -> Self {
        Self(String::from_bytes(bytes))
    }
}

impl BoundedStorable for PairKey {
    const MAX_SIZE: u32 = 32;
    const IS_FIXED_SIZE: bool = false;
}

#[cfg(test)]
mod tests {
    use ic_stable_structures::Storable;

    use crate::state::PairKey;

    #[test]
    fn pair_key_serialization() {
        let pair_key = PairKey("abdcd2332*&(\n".to_string());
        let encoded = pair_key.to_bytes();
        let decoded = PairKey::from_bytes(encoded);
        assert_eq!(pair_key, decoded);
    }
}
