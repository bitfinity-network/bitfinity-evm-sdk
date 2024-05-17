use candid::CandidType;
use serde::{Deserialize, Serialize};

use crate::{AccountInfoMap, Block, H256};


/// Describes the state of the EVM reset process.
#[derive(
    Debug, Serialize, Deserialize, CandidType
)]
 pub enum EvmResetState {
    /// Start of the reset process.
    /// It deletes all the accounts, storage, Transactions and everything else.
    Start,
    /// Add accounts to the state.
    AddAccounts(AccountInfoMap),
    /// End of the reset process.
    /// It sets the state to the given block.
    /// If the block state hash is not equal to the current state hash, it will fail.
    End(Block<H256>)
 }