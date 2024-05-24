use std::borrow::Cow;
use std::collections::BTreeMap;

use candid::Principal;
use ic_stable_structures::{Bound, Storable};

/// Storable principal. May be used as a stable storage key.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct StorablePrincipal(pub Principal);

impl StorablePrincipal {
    pub const MAX_PRINCIPAL_LENGTH_IN_BYTES: usize = 29;
}

impl Storable for StorablePrincipal {
    fn to_bytes(&self) -> Cow<'_, [u8]> {
        self.0.as_slice().into()
    }

    fn from_bytes(bytes: Cow<'_, [u8]>) -> Self {
        Self(Principal::from_slice(&bytes))
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: Self::MAX_PRINCIPAL_LENGTH_IN_BYTES as u32,
        is_fixed_size: false,
    };
}

use serde::{Deserialize, Serialize};

use crate::{Bytes, H160, U256};

/// Account full data
#[derive(Debug, candid::CandidType, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct RawAccountInfo {
    /// Account nonce.
    pub nonce: U256,
    /// Account balance.
    pub balance: U256,
    /// Account bytecode.
    pub bytecode: Option<Bytes>,
    /// Storage value for the account.
    pub storage: Vec<(U256, U256)>,
}

impl RawAccountInfo {
    /// Estimate the byte size of the account info.
    pub fn estimate_byte_size(&self) -> usize {
        const NONCE_SIZE: usize = U256::BYTE_SIZE;
        const BALANCE_SIZE: usize = U256::BYTE_SIZE;
        let bytecode_size = self.bytecode.as_ref().map(|b| b.0.len()).unwrap_or(0);
        let storage_size = U256::BYTE_SIZE * 2 * self.storage.len();

        NONCE_SIZE + BALANCE_SIZE + bytecode_size + storage_size
    }
}

/// A Map from account address to account info.
#[derive(Debug, candid::CandidType, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct AccountInfoMap {
    pub data: BTreeMap<H160, RawAccountInfo>,
}

impl AccountInfoMap {
    /// Estimate the byte size of the account info map.
    pub fn estimate_byte_size(&self) -> usize {
        const KEY_SIZE: usize = H160::BYTE_SIZE;
        let mut total_size = KEY_SIZE * self.data.len();

        for account in self.data.values() {
            total_size += account.estimate_byte_size();
        }

        total_size
    }
}

#[cfg(test)]
mod tests {

    use candid::{Decode, Encode};

    use super::*;

    #[test]
    fn test_storable_principal_roundtrip() {
        let principal_01 = Principal::from_slice(&[1; 29]);
        let principal_02 = Principal::from_slice(&[3; 24]);
        let principal_03 =
            Principal::from_text("mfufu-x6j4c-gomzb-geilq").expect("valid principal");

        let principals = vec![principal_01, principal_02, principal_03];

        for principal in principals {
            let source = StorablePrincipal(principal);
            let bytes = source.to_bytes();
            let decoded = StorablePrincipal::from_bytes(bytes);
            assert_eq!(source, decoded);
        }
    }

    #[test]
    fn test_candid_encoding_raw_account() {
        let account_info = RawAccountInfo {
            nonce: U256::from(1u64),
            balance: U256::from(2u64),
            bytecode: Some(Bytes::from(vec![1, 2, 3])),
            storage: vec![
                (U256::from(1u64), U256::from(2u64)),
                (U256::from(3u64), U256::from(4u64)),
            ],
        };

        let bytes = Encode!(&account_info).unwrap();
        let decoded = Decode!(bytes.as_slice(), RawAccountInfo).unwrap();
        assert_eq!(account_info, decoded);
    }

    #[test]
    fn test_account_info_map_roundtrip() {
        let account_info_map = AccountInfoMap {
            data: [(
                H160::from([1; 20]),
                RawAccountInfo {
                    nonce: U256::from(1u64),
                    balance: U256::from(2u64),
                    bytecode: Some(Bytes::from(vec![1, 2, 3])),
                    storage: vec![
                        (U256::from(1u64), U256::from(2u64)),
                        (U256::from(3u64), U256::from(4u64)),
                    ],
                },
            )]
            .into(),
        };

        let bytes = Encode!(&account_info_map).unwrap();
        let decoded = Decode!(bytes.as_slice(), AccountInfoMap).unwrap();
        assert_eq!(account_info_map, decoded);
    }

    #[test]
    fn test_estimate_byte_size() {
        let account_info = RawAccountInfo {
            nonce: U256::from(1u64),
            balance: U256::from(2u64),
            bytecode: Some(Bytes::from(vec![1, 2, 3])),
            storage: vec![
                (U256::from(1u64), U256::from(2u64)),
                (U256::from(3u64), U256::from(4u64)),
            ],
        };

        let account_info_map = AccountInfoMap {
            data: [
                (H160::from([1; 20]), account_info.clone()),
                (H160::from([2; 20]), account_info.clone()),
                (H160::from([3; 20]), account_info.clone()),
            ]
            .into(),
        };

        let account_info_size = account_info.estimate_byte_size();
        let account_info_map_size = account_info_map.estimate_byte_size();

        assert_eq!(account_info_size, 32 + 32 + 3 + (32 * 4));
        assert_eq!(account_info_map_size, 3 * (account_info_size + 20));
    }
}
