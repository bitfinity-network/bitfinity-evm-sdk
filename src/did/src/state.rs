use std::borrow::Cow;

use candid::CandidType;
use ic_stable_structures::{Bound, Storable};
use serde::{Deserialize, Serialize};

use crate::codec::{self, ByteChunkReader};
use crate::U256;

/// Describes basic state of an EVM account.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, CandidType)]
pub struct BasicAccount {
    /// Account balance.
    pub balance: U256,
    /// Account nonce.
    pub nonce: U256,
}

/// StableDBStorage indices information
#[derive(Debug, Clone, Serialize, CandidType, Deserialize, Eq, PartialEq)]
pub struct Indices {
    /// Index of the current block
    pub pending_block: u64,
    /// Number of block to keep history
    pub history_size: u64,
}

impl Indices {
    const STORABLE_BYTE_SIZE: usize = std::mem::size_of::<u64>() * 2;
}

impl Storable for Indices {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        let mut buf = Vec::with_capacity(Self::STORABLE_BYTE_SIZE);
        buf.extend_from_slice(&self.pending_block.to_be_bytes());
        buf.extend_from_slice(&self.history_size.to_be_bytes());
        buf.into()
    }

    fn from_bytes(bytes: Cow<'_, [u8]>) -> Self {
        let mut reader = ByteChunkReader::new(&bytes);
        let pending_block = u64::from_be_bytes(*reader.read_slice());
        let history_size = u64::from_be_bytes(*reader.read_slice());
        Self {
            pending_block,
            history_size,
        }
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: Self::STORABLE_BYTE_SIZE as _,
        is_fixed_size: true,
    };
}

/// Full information about entry
#[derive(Clone, Debug, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub struct StorageValue {
    /// Data
    pub data: Vec<u8>,
    /// Number of inserts subtracted by number of removals.
    /// May be zero for the values which were removed in past before the moment they are cleaned.
    pub rc: u32,
}

impl Storable for StorageValue {
    const BOUND: Bound = Bound::Unbounded;

    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        codec::bincode_encode(self).into()
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        codec::bincode_decode(&bytes)
    }
}

#[cfg(test)]
mod test {

    use candid::{Decode, Encode};

    use super::*;

    #[test]
    fn test_candid_basic_account() {
        let account = BasicAccount {
            balance: U256::from(1u64),
            nonce: U256::from(2u64),
        };

        let serialized = Encode!(&account).unwrap();
        let deserialized = Decode!(serialized.as_slice(), BasicAccount).unwrap();

        assert_eq!(account, deserialized);
    }

    #[test]
    fn test_storable_indices() {
        let indices = Indices {
            pending_block: 1,
            history_size: 2,
        };

        let serialized = indices.to_bytes();
        let deserialized = Indices::from_bytes(serialized);

        assert_eq!(indices, deserialized);
    }

    #[test]
    fn test_storable_storage_value() {
        let storage_value = StorageValue {
            data: vec![1, 2, 3],
            rc: 4,
        };

        let serialized = storage_value.to_bytes();
        let deserialized = StorageValue::from_bytes(serialized);

        assert_eq!(storage_value, deserialized);
    }

    #[test]
    fn test_candid_storage_value() {
        let storage_value = StorageValue {
            data: vec![1, 2, 3],
            rc: 4,
        };

        let serialized = Encode!(&storage_value).unwrap();
        let deserialized = Decode!(serialized.as_slice(), StorageValue).unwrap();

        assert_eq!(storage_value, deserialized);
    }
}
