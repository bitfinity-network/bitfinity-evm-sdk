use std::borrow::Cow;

use candid::CandidType;
use ic_stable_structures::{Bound, Storable};
use serde::{Deserialize, Serialize};

use crate::codec;
use crate::U256;

/// Describes basic state of an EVM account.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, CandidType)]
pub struct BasicAccount {
    /// Account balance.
    pub balance: U256,
    /// Account nonce.
    pub nonce: U256,
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
