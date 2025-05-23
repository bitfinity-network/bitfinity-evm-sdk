use candid::CandidType;
use serde::{Deserialize, Serialize};

use crate::{AccountInfoMap, Block, H256};

/// Describes the state of the EVM reset process.
#[derive(Debug, Serialize, Deserialize, CandidType)]
pub enum EvmResetState {
    /// Start of the reset process.
    /// It deletes all the accounts, storage, Transactions and everything else.
    Start,
    /// Add accounts to the state.
    AddAccounts(AccountInfoMap),
    /// End of the reset process.
    /// It sets the state to the given block.
    /// If the block state hash is not equal to the current state hash, it will fail.
    End(Block<H256>),
}

/// The EVM global state
#[derive(Debug, Default, Deserialize, CandidType, Clone, PartialEq, Eq, Serialize)]
pub enum EvmGlobalState {
    /// The EVM is enabled.
    /// All functions are available.
    #[default]
    Enabled,
    /// The EVM is disabled.
    /// Blocks are not processed and transactions are not executed.
    Disabled,
    /// The EVM is in staging mode.
    /// All functions are available, but the state is under testing and could be reset at any time.
    /// External users should not rely on the state.
    Staging {
        /// The maximum block number that the state can reach while in staging mode.
        max_block_number: Option<u64>,
    },
}

impl EvmGlobalState {
    /// Returns true if the EVM is enabled.
    pub fn is_enabled(&self) -> bool {
        matches!(self, EvmGlobalState::Enabled)
    }

    /// Returns true if the EVM is disabled.
    pub fn is_disabled(&self) -> bool {
        matches!(self, EvmGlobalState::Disabled)
    }

    /// Returns true if the EVM is in staging mode.
    pub fn is_staging(&self) -> bool {
        matches!(self, EvmGlobalState::Staging { .. })
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn test_evm_global_state() {
        let enabled = EvmGlobalState::Enabled;
        assert!(enabled.is_enabled());
        assert!(!enabled.is_disabled());
        assert!(!enabled.is_staging());

        let disabled = EvmGlobalState::Disabled;
        assert!(!disabled.is_enabled());
        assert!(disabled.is_disabled());
        assert!(!disabled.is_staging());

        let staging = EvmGlobalState::Staging {
            max_block_number: None,
        };
        assert!(!staging.is_enabled());
        assert!(!staging.is_disabled());
        assert!(staging.is_staging());
    }
}
