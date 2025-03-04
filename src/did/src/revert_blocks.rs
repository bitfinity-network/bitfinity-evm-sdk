use std::fmt::Display;

use candid::CandidType;
use serde::{Deserialize, Serialize};

use crate::H256;

/// Arguments for `revert_to_block` method of EVM canister
///
/// The target block is specified by the `to_block_number` field and the other fields are used to
/// verify that the caller actually knows what they are doing.
#[derive(Debug, Serialize, Deserialize, CandidType, PartialEq, Eq, Clone)]
pub struct RevertToBlockArgs {
    /// Current latest block number.
    pub from_block_number: u64,

    /// Hash of the latest block.
    pub from_block_hash: H256,

    /// Block number to revert to.
    pub to_block_number: u64,

    /// Hash of the block to revert to.
    pub to_block_hash: H256,
}

impl Display for RevertToBlockArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{from_block_number: {}, from_block_hash: {}, to_block_number: {}, to_block_hash: {}}}",
            self.from_block_number,
            self.from_block_hash,
            self.to_block_number,
            self.to_block_hash
        )
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn test_revert_to_block_args_println() {
        let args = RevertToBlockArgs {
            from_block_number: 1,
            from_block_hash: H256::from([1; 32]),
            to_block_number: 2,
            to_block_hash: H256::from([2; 32]),
        };

        assert_eq!(
            "{from_block_number: 1, from_block_hash: 0x0101010101010101010101010101010101010101010101010101010101010101, to_block_number: 2, to_block_hash: 0x0202020202020202020202020202020202020202020202020202020202020202}",
            format!("{}", args)
        );
    }
}
