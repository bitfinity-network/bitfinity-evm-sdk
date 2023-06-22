use candid::Principal;
use config::Config;
use evm::EvmCanisterImpl;
use ic_exports::stable_structures::memory_manager::MemoryId;

mod config;
pub mod evm;

pub const CONFIG_MEMORY_ID: MemoryId = MemoryId::new(80);
pub const NONCE_MEMORY_ID: MemoryId = MemoryId::new(81);
pub const ACCOUNT_MEMORY_ID: MemoryId = MemoryId::new(82);

/// State of a minter canister.
#[derive(Default)]
pub struct State {
    /// Minter canister configuration.
    pub config: Config,
    pub evm: EvmCanisterImpl,
}

impl State {
    /// Clear the state and set initial data from settings.
    pub fn reset(&mut self, settings: Settings) {
        self.config.reset(settings);
    }
}

/// State settings.
#[derive(Debug, Clone, Copy)]
pub struct Settings {
    pub owner: Principal,
    pub evmc: Principal,
}
