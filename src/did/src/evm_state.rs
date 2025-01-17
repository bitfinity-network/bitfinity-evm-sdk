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
    Staging,
}
