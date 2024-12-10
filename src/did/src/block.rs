use std::borrow::Cow;
use std::collections::HashMap;

use alloy::primitives::{keccak256, Log as AlloyLog, LogData};
use alloy::rlp::{encode_list, Decodable, Encodable, Header, PayloadView};
use bytes::BufMut;
use candid::{CandidType, Deserialize};
use ic_stable_structures::{Bound, Storable};
use serde::Serialize;
use serde_json::{json, Value};

use super::transaction::Bloom;
use super::{H160, H256, U256};
use crate::bytes::Bytes;
use crate::constant::{EIP1559_BASE_FEE_MAX_CHANGE_DENOMINATOR, EIP1559_ELASTICITY_MULTIPLIER};
use crate::error::EvmError;
use crate::hash::H64;
use crate::integer::U64;
use crate::keccak::{KECCAK_EMPTY_LIST_RLP, KECCAK_NULL_RLP};
use crate::{codec, HaltError, Transaction};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct Block<TX> {
    /// Hash of the block
    pub hash: H256,
    /// Hash of the parent
    #[serde(default, rename = "parentHash")]
    pub parent_hash: H256,
    /// Hash of the uncles/ommers
    #[serde(default, rename = "sha3Uncles")]
    pub uncles_hash: H256,
    /// Miner/author's address. None if pending.
    #[serde(default, rename = "miner")]
    pub author: H160,
    /// State root hash
    #[serde(default, rename = "stateRoot")]
    pub state_root: H256,
    /// Transactions root hash
    #[serde(default, rename = "transactionsRoot")]
    pub transactions_root: H256,
    /// Transactions receipts root hash
    #[serde(default, rename = "receiptsRoot")]
    pub receipts_root: H256,
    /// Block number. None if pending.
    pub number: U64,
    /// Gas Used
    #[serde(default, rename = "gasUsed")]
    pub gas_used: U256,
    /// Gas Limit
    #[serde(default, rename = "gasLimit")]
    pub gas_limit: U256,
    /// Extra data
    #[serde(default, rename = "extraData")]
    pub extra_data: Bytes,
    /// Logs bloom
    #[serde(rename = "logsBloom")]
    pub logs_bloom: Bloom,
    /// Timestamp
    #[serde(default)]
    pub timestamp: U256,
    /// Difficulty
    #[serde(default)]
    pub difficulty: U256,
    /// Total difficulty
    #[serde(rename = "totalDifficulty", default)]
    pub total_difficulty: U256,
    /// Seal fields
    #[serde(default, rename = "sealFields")]
    pub seal_fields: Vec<Bytes>,
    /// Uncles'/Ommers' hashes
    #[serde(default)]
    pub uncles: Vec<H256>,
    /// Transactions
    #[serde(bound = "TX: serde::Serialize + serde::de::DeserializeOwned", default)]
    pub transactions: Vec<TX>,
    /// Size in bytes
    pub size: Option<U256>,
    /// Mix Hash
    #[serde(rename = "mixHash")]
    pub mix_hash: H256,
    /// Nonce
    pub nonce: H64,
    /// Base fee per unit of gas (if past London)
    #[serde(rename = "baseFeePerGas")]
    pub base_fee_per_gas: Option<U256>,
}

impl Block<H256> {
    pub fn with_state_root(state_root: H256) -> Self {
        Self {
            hash: H256::zero(),
            parent_hash: H256::zero(),
            uncles_hash: KECCAK_EMPTY_LIST_RLP,
            author: H160::zero(),
            state_root,
            transactions_root: KECCAK_NULL_RLP,
            receipts_root: KECCAK_NULL_RLP,
            number: 0u64.into(),
            gas_used: U256::zero(),
            gas_limit: U256::zero(),
            extra_data: Bytes::default(),
            logs_bloom: Bloom::zeros(),
            timestamp: U256::zero(),
            difficulty: U256::zero(),
            total_difficulty: U256::zero(),
            seal_fields: vec![],
            uncles: vec![],
            transactions: vec![],
            size: None,
            mix_hash: H256::zero(),
            nonce: H64::zero(),
            base_fee_per_gas: None,
        }
    }

    /// Converts this block that only holds transaction hashes into a full block with `Transaction`
    pub fn into_full_block(
        self,
        transactions: Vec<Transaction>,
    ) -> Result<Block<Transaction>, EvmError> {
        let mut transactions_by_hash: HashMap<_, _> = transactions
            .into_iter()
            .map(|tx| (tx.hash.clone(), tx))
            .collect();
        let mut transactions = Vec::with_capacity(transactions_by_hash.len());
        for tx_hash in self.transactions {
            transactions.push(
                transactions_by_hash
                    .remove(&tx_hash)
                    .ok_or_else(|| EvmError::from(format!("no transaction with hash {tx_hash}")))?,
            );
        }

        Ok(Block {
            hash: self.hash,
            parent_hash: self.parent_hash,
            uncles_hash: self.uncles_hash,
            author: self.author,
            state_root: self.state_root,
            transactions_root: self.transactions_root,
            receipts_root: self.receipts_root,
            number: self.number,
            gas_used: self.gas_used,
            gas_limit: self.gas_limit,
            extra_data: self.extra_data,
            logs_bloom: self.logs_bloom,
            timestamp: self.timestamp,
            difficulty: self.difficulty,
            total_difficulty: self.total_difficulty,
            seal_fields: self.seal_fields,
            uncles: self.uncles,
            size: self.size,
            mix_hash: self.mix_hash,
            nonce: self.nonce,
            base_fee_per_gas: self.base_fee_per_gas,
            transactions,
        })
    }
}

impl<TX> Block<TX> {
    /// Encodes the block header into RLP format
    pub fn header_rlp_encoded(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        self.header_rlp_encoding(&mut buf);
        buf
    }

    /// Encodes the block header into RLP format
    pub fn header_rlp_encoding(&self, out: &mut dyn BufMut) {
        let list_header = alloy::rlp::Header {
            list: true,
            payload_length: self.header_payload_length(),
        };
        list_header.encode(out);
        self.parent_hash.encode(out);
        self.uncles_hash.encode(out);
        self.author.encode(out);
        self.state_root.encode(out);
        self.transactions_root.encode(out);
        self.receipts_root.encode(out);
        self.logs_bloom.encode(out);
        self.difficulty.encode(out);
        self.number.encode(out);
        self.gas_limit.encode(out);
        self.gas_used.encode(out);
        self.timestamp.encode(out);
        self.extra_data.encode(out);
        self.mix_hash.encode(out);
        self.nonce.encode(out);

        // Encode all the fork specific fields
        if let Some(ref base_fee) = self.base_fee_per_gas {
            base_fee.encode(out);
        }
    }

    /// Returns the length of the header payload for rlp encoding
    pub fn header_payload_length(&self) -> usize {
        let mut length = 0;
        length += self.parent_hash.length();
        length += self.uncles_hash.length();
        length += self.author.length();
        length += self.state_root.length();
        length += self.transactions_root.length();
        length += self.receipts_root.length();
        length += self.logs_bloom.length();
        length += self.difficulty.length();
        length += self.number.length();
        length += self.gas_limit.length();
        length += self.gas_used.length();
        length += self.timestamp.length();
        length += self.extra_data.length();
        length += self.mix_hash.length();
        length += self.nonce.length();

        if let Some(base_fee) = &self.base_fee_per_gas {
            // Adding base fee length if it exists.
            length += base_fee.length();
        }

        length
    }
}

impl Default for Block<H256> {
    fn default() -> Self {
        Block::with_state_root(KECCAK_NULL_RLP)
    }
}

impl From<Block<Transaction>> for Block<H256> {
    fn from(block: Block<Transaction>) -> Self {
        Self {
            hash: block.hash,
            parent_hash: block.parent_hash,
            uncles_hash: block.uncles_hash,
            author: block.author,
            state_root: block.state_root,
            transactions_root: block.transactions_root,
            receipts_root: block.receipts_root,
            number: block.number,
            gas_used: block.gas_used,
            gas_limit: block.gas_limit,
            extra_data: block.extra_data,
            logs_bloom: block.logs_bloom,
            timestamp: block.timestamp,
            difficulty: block.difficulty,
            total_difficulty: block.total_difficulty,
            seal_fields: block.seal_fields,
            uncles: block.uncles,
            transactions: block
                .transactions
                .iter()
                .map(|tx| tx.hash.clone())
                .collect(),
            size: block.size,
            mix_hash: block.mix_hash,
            nonce: block.nonce,
            base_fee_per_gas: block.base_fee_per_gas,
        }
    }
}

/// Calculate the hash of a block
pub fn calculate_block_hash<T>(block: &Block<T>) -> H256 {
    keccak256(block.header_rlp_encoded()).into()
}

/// Calculate the size of a block in bytes considering all of its transactions
pub fn calculate_block_size<'a>(
    block: &Block<H256>,
    transactions: impl Iterator<Item = &'a Transaction>,
) -> U256 {
    let block_size = block.to_bytes().len();
    let transactions_size: usize = transactions.map(|x| x.to_bytes().len()).sum();

    // If `size` is still `None` we need to consider the size it would take once set for block
    let size_field_size = match block.size {
        None => U256::BYTE_SIZE,
        Some(_) => 0,
    };

    U256::from((block_size + transactions_size + size_field_size) as u64)
}

/// Calculate base fee for next block. [EIP-1559](https://github.com/ethereum/EIPs/blob/master/EIPS/eip-1559.md) spec
pub fn calculate_next_block_base_fee(
    parent_gas_used: &U256,
    parent_gas_limit: &U256,
    parent_base_fee: &U256,
) -> U256 {
    let gas_target: U256 = parent_gas_limit
        .checked_div(&U256::from(EIP1559_ELASTICITY_MULTIPLIER))
        .unwrap_or_default();

    if parent_gas_used == &gas_target {
        return parent_base_fee.clone();
    }

    let gas_used_delta = if parent_gas_used > &gas_target {
        parent_gas_used.checked_sub(&gas_target)
    } else {
        gas_target.checked_sub(parent_gas_used)
    }
    .unwrap_or_default();

    let base_fee_per_gas_delta = parent_base_fee
        .checked_mul(&gas_used_delta)
        .and_then(|x| x.checked_div(&gas_target))
        .and_then(|x| x.checked_div(&U256::from(EIP1559_BASE_FEE_MAX_CHANGE_DENOMINATOR)))
        .unwrap_or_default();

    if parent_gas_used > &gas_target {
        let base_fee_delta = std::cmp::max(U256::from(1u64), base_fee_per_gas_delta);
        parent_base_fee + &base_fee_delta
    } else {
        parent_base_fee
            .checked_sub(&base_fee_per_gas_delta)
            .unwrap_or_default()
    }
}

impl Storable for Block<H256> {
    const BOUND: Bound = Bound::Unbounded;

    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        codec::encode(self).into()
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        codec::decode(&bytes)
    }
}

impl Decodable for Block<Transaction> {
    fn decode(buf: &mut &[u8]) -> alloy::rlp::Result<Self> {
        let payload = Header::decode_raw(buf)?;
        match payload {
            PayloadView::List(items) => {
                let mut header = items[0];
                let mut block = match Header::decode_raw(&mut header)? {
                    PayloadView::List(mut header_items) => {
                        let item_count = header_items.len();
                        Self {
                            parent_hash: alloy::primitives::B256::decode(&mut header_items[0])?
                                .into(),
                            uncles_hash: alloy::primitives::B256::decode(&mut header_items[1])?
                                .into(),
                            author: alloy::primitives::Address::decode(&mut header_items[2])?
                                .into(),
                            state_root: alloy::primitives::B256::decode(&mut header_items[3])?
                                .into(),
                            transactions_root: alloy::primitives::B256::decode(
                                &mut header_items[4],
                            )?
                            .into(),
                            receipts_root: alloy::primitives::B256::decode(&mut header_items[5])?
                                .into(),
                            logs_bloom: alloy::primitives::Bloom::decode(&mut header_items[6])?
                                .into(),
                            difficulty: alloy::primitives::U256::decode(&mut header_items[7])?
                                .into(),
                            number: alloy::primitives::U64::decode(&mut header_items[8])?.into(),
                            gas_limit: alloy::primitives::U256::decode(&mut header_items[9])?
                                .into(),
                            gas_used: alloy::primitives::U256::decode(&mut header_items[10])?
                                .into(),
                            timestamp: alloy::primitives::U256::decode(&mut header_items[11])?
                                .into(),
                            extra_data: alloy::primitives::Bytes::decode(&mut header_items[12])?
                                .into(),
                            mix_hash: alloy::primitives::B256::decode(&mut header_items[13])?
                                .into(),
                            nonce: alloy::primitives::B64::decode(&mut header_items[14])?.into(),
                            hash: Default::default(),
                            total_difficulty: Default::default(),
                            seal_fields: Vec::new(),
                            uncles: Vec::new(),
                            transactions: Vec::new(),
                            size: None,
                            base_fee_per_gas: if item_count > 15 {
                                Some(alloy::primitives::U256::decode(&mut header_items[15])?.into())
                            } else {
                                None
                            },
                        }
                    }
                    PayloadView::String(_) => return Err(alloy::rlp::Error::UnexpectedString),
                };

                let mut transactions = items[1];
                match Header::decode_raw(&mut transactions)? {
                    PayloadView::List(transactions_header) => {
                        for mut transaction in transactions_header {
                            let tx = alloy::consensus::TxEnvelope::decode(&mut transaction)?;
                            block.transactions.push(tx.into());
                        }
                    }
                    PayloadView::String(_) => return Err(alloy::rlp::Error::UnexpectedString),
                }
                Ok(block)
            }
            PayloadView::String(_) => Err(alloy::rlp::Error::UnexpectedString),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, CandidType)]
pub struct TransactionExecutionLog {
    /// The contract that emitted the log
    pub address: H160,

    /// Topics: Array of 0 to 4 32 Bytes of indexed log arguments.
    /// (In solidity: The first topic is the hash of the signature of the event
    /// (e.g. `Deposit(address,bytes32,uint256)`), except you declared the event
    /// with the anonymous specifier.)
    pub topics: Vec<H256>,

    /// Data
    pub data: Bytes,
}

impl Encodable for TransactionExecutionLog {
    fn encode(&self, out: &mut dyn bytes::BufMut) {
        let enc: [&dyn Encodable; 3] = [&self.address, &self.topics, &self.data];
        encode_list::<_, dyn Encodable>(&enc, out);
    }
}

impl From<AlloyLog> for TransactionExecutionLog {
    fn from(log: AlloyLog) -> Self {
        Self {
            address: log.address.into(),
            topics: log.topics().iter().map(|h| (*h).into()).collect(),
            data: log.data.data.into(),
        }
    }
}

impl From<TransactionExecutionLog> for AlloyLog {
    fn from(log: TransactionExecutionLog) -> Self {
        Self {
            address: log.address.into(),
            data: LogData::new_unchecked(
                log.topics.into_iter().map(|h| h.into()).collect(),
                log.data.into(),
            ),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, CandidType)]
pub enum ExeResult {
    /// Returned successfully
    Success {
        gas_used: U256,
        logs: Vec<TransactionExecutionLog>,
        logs_bloom: Box<Bloom>,
        output: TransactOut,
    },
    /// Reverted by `REVERT` opcode that doesn't spend all gas.
    Revert {
        revert_message: Option<String>,
        gas_used: U256,
        output: Bytes,
    },
    /// Reverted for various reasons and spend all gas.
    Halt {
        error: HaltError,
        /// Halting will spend all the gas, and will be equal to gas_limit.
        gas_used: U256,
    },
}

impl ExeResult {
    pub fn success(
        gas_used: U256,
        output: TransactOut,
        logs: Vec<TransactionExecutionLog>,
    ) -> Self {
        let logs_bloom = Bloom::from_logs(&logs);
        Self::Success {
            gas_used,
            logs,
            logs_bloom: Box::new(logs_bloom),
            output,
        }
    }

    pub fn gas_used(&self) -> &U256 {
        match self {
            ExeResult::Success { gas_used, .. } => gas_used,
            ExeResult::Revert { gas_used, .. } => gas_used,
            ExeResult::Halt { gas_used, .. } => gas_used,
        }
    }
}

impl Storable for ExeResult {
    const BOUND: Bound = Bound::Unbounded;

    fn to_bytes(&self) -> std::borrow::Cow<'_, [u8]> {
        codec::encode(self).into()
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        codec::decode(&bytes)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, CandidType)]
pub enum TransactOut {
    None,
    Call(Vec<u8>),
    Create(Vec<u8>, Option<H160>),
}

impl Default for TransactOut {
    fn default() -> Self {
        Self::Call(vec![])
    }
}

/// enum representing the BlockResult

#[derive(Debug, CandidType, Deserialize, PartialEq, Eq, Serialize)]
pub enum BlockResult {
    /// No block found
    NoBlockFound,

    /// Block with transactions
    WithTransaction(Block<Transaction>),

    /// Block with hashes
    WithHash(Block<H256>),
}

impl BlockResult {
    pub fn to_json(&self) -> Value {
        match self {
            BlockResult::WithHash(block) => json!(block),
            BlockResult::WithTransaction(block) => json!(block),
            BlockResult::NoBlockFound => Value::Null,
        }
    }
}

#[cfg(test)]
mod test {

    use std::fmt::Debug;
    use std::str::FromStr;

    use candid::{Decode, Encode};

    use super::*;
    use crate::test_utils::{read_all_files_to_json, test_candid_roundtrip, test_json_roundtrip};
    use crate::BlockId;

    #[test]
    fn test_storable_block() {
        let mut block = Block {
            author: alloy::primitives::Address::random().into(),
            number: rand::random::<u64>().into(),
            ..Default::default()
        };
        block.hash = calculate_block_hash(&block);

        let serialized = block.to_bytes();
        let deserialized = Block::from_bytes(serialized);

        assert_eq!(block, deserialized);
    }

    #[test]
    fn test_candid_encoding_block() {
        let block = Block::<H256>::default();

        let res0 = Encode!(&block).unwrap();
        let res = Decode!(res0.as_slice(), Block::<H256>).unwrap();

        assert_eq!(block, res);
    }

    #[test]
    fn test_parse_real_blocks_from_ethereum() {
        let jsons = read_all_files_to_json("./tests/resources/json/block");

        for (hash, value) in jsons {
            println!("Check block {}", hash);
            let value = value.get("result").unwrap().to_owned();
            let block: Block<H256> = serde_json::from_value(value.clone()).unwrap();

            let block_to_value = serde_json::to_value(block.clone()).unwrap();
            let block_from_value: Block<H256> =
                serde_json::from_value(block_to_value.clone()).unwrap();
            assert_eq!(block_from_value, block);

            let calculated_block_hash = calculate_block_hash(&block);
            assert_eq!(
                alloy::primitives::B256::from_str(&hash).unwrap(),
                calculated_block_hash.0
            );
        }
    }

    fn create_transaction(gas_price: Option<U256>, chain_id: u64) -> Transaction {
        let mut tx = Transaction {
            from: alloy::primitives::Address::from_slice(&[0u8; 20]).into(),
            to: None,
            nonce: U256::zero(),
            value: U256::zero(),
            gas: 20u64.into(),
            gas_price,
            input: vec![].into(),
            chain_id: Some(chain_id.into()),
            ..Default::default()
        };
        tx.hash = alloy::primitives::B256::random().into();
        tx
    }

    #[test]
    fn test_block_result() {
        let block = Block::<Transaction> {
            author: alloy::primitives::Address::random().into(),
            number: U64::from(rand::random::<u64>()),
            logs_bloom: Bloom(alloy::primitives::Bloom::from_slice(&[4u8; 256])),
            nonce: alloy::primitives::B64::random().into(),
            transactions: vec![create_transaction(
                Some(U256::from(rand::random::<u64>())),
                1,
            )],
            mix_hash: alloy::primitives::B256::random().into(),
            hash: Default::default(),
            parent_hash: alloy::primitives::B256::random().into(),
            uncles_hash: alloy::primitives::B256::random().into(),
            state_root: alloy::primitives::B256::random().into(),
            transactions_root: alloy::primitives::B256::random().into(),
            receipts_root: alloy::primitives::B256::random().into(),
            gas_used: U256::from(rand::random::<u64>()),
            gas_limit: U256::from(rand::random::<u64>()),
            extra_data: Default::default(),
            timestamp: U256::from(rand::random::<u64>()),
            difficulty: U256::from(rand::random::<u64>()),
            total_difficulty: Default::default(),
            seal_fields: Vec::new(),
            uncles: Vec::new(),
            size: None,
            base_fee_per_gas: Some(U256::from(rand::random::<u64>())),
        };

        let block_result = BlockResult::WithTransaction(block);

        let encoded_value = serde_json::json!(&block_result);

        let decoded_value: BlockResult = serde_json::from_value(encoded_value).unwrap();

        assert_eq!(block_result, decoded_value);
    }

    #[test]
    fn should_calc_block_base_fee_when_gas_used_eq_gas_target() {
        assert_eq!(
            calculate_next_block_base_fee(
                &2_u64.into(),
                &4_u64.into(), // gas target 2
                &1_u64.into()
            ),
            U256::from(1u64)
        );
    }

    #[test]
    fn should_calc_block_base_fee_when_gas_used_is_gt_gas_target() {
        assert_eq!(
            calculate_next_block_base_fee(
                &10_u64.into(),
                &4_u64.into(), // gas target 2
                &1_u64.into()
            ),
            U256::from(2_u64)
        );
    }

    #[test]
    fn should_calc_block_base_fee_eq_to_base_fee_when_gas_used_is_lt_gas_target_and_sub_overflows()
    {
        let base_fee = U256::from(100_u64);
        assert_eq!(
            calculate_next_block_base_fee(
                &4_u64.into(),
                &10_u64.into(), // gas target 5
                &base_fee
            ),
            U256::from(98u64) // = 100 - 0.125 * ((5-4) / 5) * 100
        );
    }

    #[test]
    fn should_calc_block_base_fee_eq_to_sum_of_one_and_base_fee_when_gas_limit_is_zero() {
        let gas_used = U256::from(5_u64);
        let base_fee = U256::from(100_u64);
        let expected = &U256::from(1u64) + &base_fee;
        assert_eq!(
            calculate_next_block_base_fee(
                &gas_used,
                &U256::zero(), // gas target 0
                &base_fee
            ),
            expected
        );
    }

    #[test]
    fn should_calc_base_fee_for_zero_used_gas() {
        let gas_used = U256::from(0_u64);
        let base_fee = U256::from(100_u64);
        let expected = U256::from(88u64); // 100 - 0.125 * 100
        assert_eq!(
            calculate_next_block_base_fee(
                &gas_used,
                &U256::from(100u64), // gas target 0
                &base_fee
            ),
            expected
        );
    }

    #[test]
    fn should_calc_base_fee_with_arithmetic_overflow() {
        let gas_used = U256::from(2000_u64);
        let gas_limit = U256::from(2010_u64);
        let mut base_fee = U256::from(100_u64);

        for _ in 0..10000 {
            base_fee = calculate_next_block_base_fee(&gas_used, &gas_limit, &base_fee);
        }
    }

    fn check_serialization_roundtrip<T>(val: &T)
    where
        for<'a> T: CandidType + Serialize + Deserialize<'a> + Eq + Debug,
    {
        test_json_roundtrip(val);
        test_candid_roundtrip(val);
    }

    #[test]
    fn block_id_should_roundtrip_serialization() {
        check_serialization_roundtrip(&BlockId::BlockHash(H256::from_slice(&[42; 32])));
        check_serialization_roundtrip(&BlockId::BlockNumber(crate::BlockNumber::Earliest));
        check_serialization_roundtrip(&BlockId::BlockNumber(crate::BlockNumber::Latest));
        check_serialization_roundtrip(&BlockId::BlockNumber(crate::BlockNumber::Pending));
        check_serialization_roundtrip(&BlockId::BlockNumber(crate::BlockNumber::Number(
            42u64.into(),
        )));
    }

    #[test]
    fn test_block_into_full_block() {
        let transactions = vec![
            create_transaction(Some(U256::from(rand::random::<u64>())), 1),
            create_transaction(Some(U256::from(rand::random::<u64>())), 2),
            create_transaction(Some(U256::from(rand::random::<u64>())), 3),
        ];

        let tx_hashes = transactions
            .iter()
            .map(|tx| tx.hash.clone())
            .collect::<Vec<_>>();

        let block = Block::<H256> {
            author: alloy::primitives::Address::random().into(),
            number: U64::from(rand::random::<u64>()),
            logs_bloom: Bloom(alloy::primitives::Bloom::from_slice(&[4u8; 256])),
            nonce: alloy::primitives::B64::random().into(),
            transactions: tx_hashes,
            mix_hash: alloy::primitives::B256::random().into(),
            hash: Default::default(),
            parent_hash: alloy::primitives::B256::random().into(),
            uncles_hash: alloy::primitives::B256::random().into(),
            state_root: alloy::primitives::B256::random().into(),
            transactions_root: alloy::primitives::B256::random().into(),
            receipts_root: alloy::primitives::B256::random().into(),
            gas_used: U256::from(rand::random::<u64>()),
            gas_limit: U256::from(rand::random::<u64>()),
            extra_data: Default::default(),
            timestamp: U256::from(rand::random::<u64>()),
            difficulty: U256::from(rand::random::<u64>()),
            total_difficulty: Default::default(),
            seal_fields: Vec::new(),
            uncles: Vec::new(),
            size: None,
            base_fee_per_gas: Some(U256::from(rand::random::<u64>())),
        };

        let full_block = block.clone().into_full_block(transactions.clone()).unwrap();
        assert_eq!(full_block.transactions, transactions);

        let block_from_full_block: Block<H256> = full_block.into();
        assert_eq!(block_from_full_block, block);
    }

    #[test]
    fn test_block_into_full_block_different_order() {
        let transactions = vec![
            create_transaction(Some(U256::from(rand::random::<u64>())), 1),
            create_transaction(Some(U256::from(rand::random::<u64>())), 2),
            create_transaction(Some(U256::from(rand::random::<u64>())), 3),
        ];

        let tx_hashes = transactions
            .iter()
            .map(|tx| tx.hash.clone())
            .collect::<Vec<_>>();

        let block = Block::<H256> {
            author: alloy::primitives::Address::random().into(),
            number: U64::from(rand::random::<u64>()),
            logs_bloom: Bloom(alloy::primitives::Bloom::from_slice(&[4u8; 256])),
            nonce: alloy::primitives::B64::random().into(),
            transactions: tx_hashes,
            mix_hash: alloy::primitives::B256::random().into(),
            hash: Default::default(),
            parent_hash: alloy::primitives::B256::random().into(),
            uncles_hash: alloy::primitives::B256::random().into(),
            state_root: alloy::primitives::B256::random().into(),
            transactions_root: alloy::primitives::B256::random().into(),
            receipts_root: alloy::primitives::B256::random().into(),
            gas_used: U256::from(rand::random::<u64>()),
            gas_limit: U256::from(rand::random::<u64>()),
            extra_data: Default::default(),
            timestamp: U256::from(rand::random::<u64>()),
            difficulty: U256::from(rand::random::<u64>()),
            total_difficulty: Default::default(),
            seal_fields: Vec::new(),
            uncles: Vec::new(),
            size: None,
            base_fee_per_gas: Some(U256::from(rand::random::<u64>())),
        };

        // Check if we pass transactions in the wrong order we still get the order corresponding to hashes
        let full_block = block
            .clone()
            .into_full_block(transactions.clone().into_iter().rev().collect())
            .unwrap();
        assert_eq!(full_block.transactions, transactions);

        let block_from_full_block: Block<H256> = full_block.into();
        assert_eq!(block_from_full_block, block);
    }
}
