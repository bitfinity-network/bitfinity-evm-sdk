use candid::{CandidType, Deserialize};
use serde::Serialize;

use crate::H256;

#[derive(CandidType, Serialize, Deserialize, Debug)]
/// Arguments to `validate_unconfirmed_blocks` function.
pub struct ValidateUnconfirmedBlocksArgs {
    pub block_number: u64,
    pub block_hash: H256,
    pub state_root_hash: H256,
    pub transaction_root_hash: H256,
    pub receipts_root_hash: H256,
}
