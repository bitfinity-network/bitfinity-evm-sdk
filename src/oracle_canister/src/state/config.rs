use std::borrow::Cow;
use std::cell::RefCell;

use candid::{CandidType, Decode, Deserialize, Encode, Principal};
use ic_stable_structures::{StableCell, Storable};

use super::Settings;
use crate::state::CONFIG_MEMORY_ID;

/// Minter canister configuration.
#[derive(Default)]
pub struct Config {}

impl Config {
    /// Clear configuration and initialize it with data from `settings`.
    pub fn reset(&mut self, settings: Settings) {
        let new_data = ConfigData {
            owner: settings.owner,
            evmc_principal: settings.evmc_principal,
        };
        self.update_data(|data| *data = new_data);
    }

    /// Returns principal of canister owner.
    pub fn get_owner(&self) -> Principal {
        self.with_data(|data| data.get().owner)
    }

    /// Sets a new principal for canister owner.
    pub fn set_owner(&mut self, owner: Principal) {
        self.update_data(|data| data.owner = owner);
    }

    /// Returns principal of EVM canister with which the minter canister works.
    pub fn get_evmc_principal(&self) -> Principal {
        self.with_data(|data| data.get().evmc_principal)
    }

    /// Sets principal of EVM canister with which the minter canister works.
    pub fn set_evmc_principal(&mut self, evmc: Principal) {
        self.update_data(|data| data.evmc_principal = evmc);
    }

    fn with_data<F, T>(&self, f: F) -> T
    where
        F: FnOnce(&StableCell<ConfigData>) -> T,
    {
        CONFIG_CELL.with(|cell| f(&mut cell.borrow()))
    }

    fn with_mut_data<F, T>(&mut self, f: F) -> T
    where
        F: FnOnce(&mut StableCell<ConfigData>) -> T,
    {
        CONFIG_CELL.with(|cell| f(&mut cell.borrow_mut()))
    }

    fn update_data<F, T>(&mut self, f: F) -> T
    where
        F: FnOnce(&mut ConfigData) -> T,
    {
        self.with_mut_data(|data| {
            let mut old_data = *data.get();
            let result = f(&mut old_data);
            data.set(old_data)
                .expect("failed to update config stable memory data");
            result
        })
    }
}

#[derive(Debug, Clone, Copy, Deserialize, CandidType, PartialEq, Eq)]
pub struct ConfigData {
    pub owner: Principal,
    pub evmc_principal: Principal,
}

impl Default for ConfigData {
    fn default() -> Self {
        Self {
            owner: Principal::anonymous(),
            evmc_principal: Principal::anonymous(),
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
    static CONFIG_CELL: RefCell<StableCell<ConfigData>> = {
        RefCell::new(StableCell::new(CONFIG_MEMORY_ID, ConfigData::default())
            .expect("stable memory config initialization failed"))
    };
}

pub fn encode(item: &impl CandidType) -> Vec<u8> {
    Encode!(item).expect("failed to encode item to candid")
}

pub fn decode<'a, T: CandidType + Deserialize<'a>>(bytes: &'a [u8]) -> T {
    Decode!(bytes, T).expect("failed to decode item from candid")
}

#[cfg(test)]
mod tests {
    use candid::Principal;
    use ic_exports::ic_kit::MockContext;
    use ic_stable_structures::Storable;

    use super::Config;
    use crate::state::config::ConfigData;
    use crate::state::Settings;

    fn get_config() -> Config {
        MockContext::new().inject();
        let mut config = Config::default();
        config.reset(Settings::default());
        config
    }

    #[test]
    fn config_serialization() {
        let config = ConfigData {
            owner: Principal::anonymous(),
            evmc_principal: Principal::anonymous(),
        };
        let encoded = config.to_bytes();
        let decoded = ConfigData::from_bytes(encoded);
        assert_eq!(config, decoded);
    }

    #[test]
    fn reset_should_update_config() {
        let mut config = get_config();

        let settings = Settings {
            owner: Principal::management_canister(),
            evmc_principal: Principal::anonymous(),
        };

        config.reset(settings);

        assert_eq!(config.get_owner(), settings.owner);
        assert_eq!(config.get_evmc_principal(), settings.evmc_principal);
    }

    #[test]
    fn config_data_stored_after_set() {
        let mut config = get_config();

        config.set_owner(Principal::management_canister());
        config.set_evmc_principal(Principal::management_canister());

        assert_eq!(config.get_owner(), Principal::management_canister());
        assert_eq!(
            config.get_evmc_principal(),
            Principal::management_canister()
        );
    }
}
