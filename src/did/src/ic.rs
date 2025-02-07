use std::borrow::Cow;
use std::collections::BTreeMap;

use candid::CandidType;
use ic_stable_structures::Storable;
use serde::{Deserialize, Serialize};

use crate::{codec, Bytes, H160, H256, U256};

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
#[derive(Debug, Default, candid::CandidType, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct AccountInfoMap {
    pub data: BTreeMap<H160, RawAccountInfo>,
}

impl AccountInfoMap {
    /// Create a new account info map.
    pub fn new() -> Self {
        Self::default()
    }

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

impl<D: Into<BTreeMap<H160, RawAccountInfo>>> From<D> for AccountInfoMap {
    fn from(data: D) -> Self {
        Self { data: data.into() }
    }
}

/// Contains the stats for the evm
#[derive(Debug, Clone, CandidType, Eq, PartialEq, Deserialize)]
pub struct EvmStats {
    /// This is the number of the pending transaction count
    pub pending_transactions_count: usize,
    /// Returns a vec of the transactions in the pool
    pub pending_transactions: Vec<H256>,
    /// Latest Block number
    pub block_number: u64,
    /// The CHAIN_ID for the evm
    pub chain_id: u64,
    /// This is the hash of all account balances, contract storage etc
    pub state_root: H256,
    /// Amount of Cycles that the canister has
    pub cycles: u128,
    /// The gas limit for the block
    pub block_gas_limit: u64,
    /// The total number of blocks in the history
    pub blocks_history_count: u64,
    /// The total number of receipts in the history
    pub receipts_history_count: u64,
    /// The total number of transactions in the history
    pub transactions_history_count: u64,
    /// The oldest version in the trie
    pub oldest_block_in_trie_history: u64,
}

/// The limits for the blockchain storage
#[derive(Debug, Copy, Clone, CandidType, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlockchainStorageLimits {
    /// The maximum number of the blocks in the storage
    pub blocks_max_history_size: u64,
    /// The maximum number of the transactions and receipts in the storage
    pub transactions_and_receipts_max_history_size: u64,
}

/// Information about the blockchain
#[derive(Debug, Clone, CandidType, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlockchainBlockInfo {
    /// The number of the first block in the blockchain
    pub earliest_block_number: u64,
    /// The number of the latest block in the blockchain
    pub latest_block_number: u64,
    /// The number of the safe block in the blockchain
    pub safe_block_number: u64,
    /// The number of the finalized block in the blockchain
    pub finalized_block_number: u64,
    /// The number of the pending block in the blockchain
    pub pending_block_number: u64,
}

/// Strategy for confirming a block.
/// When a block is confirmed, it becomes `safe`.
#[derive(Debug, Clone, CandidType, Serialize, Deserialize, PartialEq, Eq)]
pub enum BlockConfirmationStrategy {
    /// The block does not require any particular confirmation,
    /// it is always considered safe.
    None,

    /// The block requires a proof of work to be considered safe.
    /// The block is dropped if the proof of work is not provided in time.
    HashDropOnTimeout {
        /// The number of seconds to wait before dropping the block.
        /// If the block is not confirmed by then, it is dropped.
        timeout_secs: u64,
    },
}

impl Storable for BlockConfirmationStrategy {
    const BOUND: ic_stable_structures::Bound = ic_stable_structures::Bound::Unbounded;

    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        codec::encode(self).into()
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        codec::decode(&bytes)
    }
}

/// Data required to confirm a block and mark it `safe`.
#[derive(Default, Debug, Clone, CandidType, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlockConfirmationData {
    /// the block number
    pub block_number: u64,
    /// Hash of the block
    pub hash: H256,
    /// State root of the block
    pub state_root: H256,
    /// Transactions root of the block
    pub transactions_root: H256,
    /// Receipts root of the block
    pub receipts_root: H256,
    /// Proof of work of the block provided by the validator
    pub proof_of_work: H256,
}

/// Result of confirming a block.
#[derive(Debug, Clone, CandidType, Serialize, Deserialize, PartialEq, Eq)]
pub enum BlockConfirmationResult {
    /// The block is confirmed and is now safe.
    Confirmed,
    /// The block is not confirmed and is not safe.
    NotConfirmed,
    /// The block is already confirmed and is safe.
    AlreadyConfirmed,
}

#[cfg(test)]
mod tests {

    use candid::{Decode, Encode};

    use super::*;

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

    #[test]
    fn test_storable_block_confirmation_strategy() {
        let strategy = BlockConfirmationStrategy::HashDropOnTimeout { timeout_secs: 10 };

        let serialized = strategy.to_bytes();
        let deserialized = BlockConfirmationStrategy::from_bytes(serialized);

        assert_eq!(strategy, deserialized);
    }
}
