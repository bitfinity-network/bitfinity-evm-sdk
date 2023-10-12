use std::borrow::Cow;
use std::cell::RefCell;

use candid::{CandidType, Decode, Deserialize, Encode, Principal};
use ic_stable_structures::{get_memory_by_id, CellStructure, StableCell, Storable};

use super::Settings;
use crate::memory::{MemoryType, MEMORY_MANAGER};
use crate::state::CONFIG_MEMORY_ID;

/// Minter canister configuration.
#[derive(Default)]
pub struct Config {}

impl Config {
    /// Clear configuration and initialize it with data from `settings`.
    pub fn reset(&mut self, settings: Settings) {
        let new_data = ConfigData {
            owner: settings.owner,
            evm: settings.evm,
        };
        CONFIG_CELL.with(|cell| {
            cell.borrow_mut()
                .set(new_data)
                .expect("failed to update config stable memory data")
        })
    }

    /// Returns principal of canister owner.
    pub fn get_owner(&self) -> Principal {
        CONFIG_CELL.with(|cell| cell.borrow().get().owner)
    }

    /// Returns principal of evm canister id.
    pub fn get_evm_canister_id(&self) -> Principal {
        CONFIG_CELL.with(|cell| cell.borrow().get().evm)
    }

    /// Sets a new principal for canister owner.
    pub fn set_owner(&mut self, owner: Principal) {
        let evm = self.get_evm_canister_id();
        CONFIG_CELL
            .with(|cell| cell.borrow_mut().set(ConfigData { owner, evm }))
            .expect("failed to update config stable memory data")
    }

    /// Sets a new principal for evm canister id.
    pub fn set_evm(&mut self, evm: Principal) {
        let owner: Principal = self.get_owner();
        CONFIG_CELL
            .with(|cell| cell.borrow_mut().set(ConfigData { owner, evm }))
            .expect("failed to update config stable memory data")
    }
}

#[derive(Debug, Clone, Copy, Deserialize, CandidType, PartialEq, Eq)]
pub struct ConfigData {
    pub owner: Principal,
    pub evm: Principal,
}

impl Default for ConfigData {
    fn default() -> Self {
        Self {
            owner: Principal::anonymous(),
            evm: Principal::anonymous(),
        }
    }
}

impl Storable for ConfigData {
    fn to_bytes(&self) -> Cow<'_, [u8]> {
        encode(&self).into()
    }

    fn from_bytes(bytes: Cow<'_, [u8]>) -> Self {
        decode(bytes.as_ref())
    }
}

thread_local! {
    static CONFIG_CELL: RefCell<StableCell<ConfigData, MemoryType>> = {
        RefCell::new(StableCell::new(get_memory_by_id(&MEMORY_MANAGER, CONFIG_MEMORY_ID), ConfigData::default())
            .expect("stable memory config initialization failed"))
    };
}

pub fn encode(item: &impl CandidType) -> Vec<u8> {
    Encode!(item).expect("failed to encode item to candid")
}

pub fn decode<'a, T: CandidType + Deserialize<'a>>(bytes: &'a [u8]) -> T {
    Decode!(bytes, T).expect("failed to decode item from candid")
}
