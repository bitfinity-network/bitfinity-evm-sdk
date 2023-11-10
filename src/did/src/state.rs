use std::io::Cursor;
use std::{fmt, mem};

use candid::CandidType;
use serde::{Deserialize, Serialize};

use crate::U256;

/// Describes basic state of an EVM account.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, CandidType)]
pub struct BasicAccount {
    /// Account balance.
    pub balance: U256,
    /// Account nonce.
    pub nonce: U256,
}

/// Action to update key-value state
#[derive(Debug, CandidType, Deserialize, Clone)]
pub enum StateUpdateAction<K, V> {
    Removed { key: K },
    Replace { key: K, value: V },
}

/// StableDBStorage indices information
#[derive(Debug, Clone, Serialize, CandidType, Deserialize, Eq, PartialEq)]
pub struct Indices {
    /// Index of the current block
    pub pending_block: u64,
    /// Number of block to keep history
    pub history_size: u64,
}

/// Full information about entry
#[derive(Clone, CandidType, Deserialize)]
pub struct FullStorageValue {
    /// Data
    pub data: Vec<u8>,
    /// Number of inserts subtracted by number of removals.
    /// May be zero for the values which were removed in past before the moment they are cleaned.
    pub ref_count: u32,
    /// Index of the block when the item was removed last time (ref counter set to zeo)
    pub removed_at_block: u64,
}

impl fmt::Debug for FullStorageValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FullStorageValue")
            .field("data_len", &self.data.len())
            .field("ref_count", &self.ref_count)
            .field("removed_at_block", &self.removed_at_block)
            .finish()
    }
}

impl FullStorageValue {
    pub fn hash(&self) -> u128 {
        let mut all_data = Vec::with_capacity(
            self.data.len()
                + mem::size_of_val(&self.ref_count)
                + mem::size_of_val(&self.removed_at_block),
        );
        all_data.extend(&self.data);
        all_data.extend(self.ref_count.to_le_bytes());
        all_data.extend(self.removed_at_block.to_le_bytes());

        murmur3::murmur3_x86_128(&mut Cursor::new(&all_data), 0).expect("should calculate hash")
    }
}
