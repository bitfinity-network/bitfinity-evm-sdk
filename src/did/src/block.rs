use std::borrow::Cow;

use candid::{CandidType, Deserialize};
use ethers_core::types::Log as EthersLog;
use ic_stable_structures::{ChunkSize, SlicedStorable, Storable};
use rlp::{Decodable, DecoderError, Encodable, Rlp, RlpStream};
use serde::Serialize;
use serde_json::{json, Value};

use super::transaction::Bloom;
use super::{H160, H256, U256};
use crate::bytes::Bytes;
use crate::hash::H64;
use crate::integer::U64;
use crate::keccak::{keccak_hash, KECCAK_EMPTY_LIST_RLP, KECCAK_NULL_RLP};
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
    #[serde(rename = "totalDifficulty")]
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

impl Encodable for Block<Transaction> {
    fn rlp_append(&self, s: &mut RlpStream) {
        s.begin_list(3); // block header, transactions, other block headers

        // Block header
        block_header_rlp(self, s);

        // Block transactions
        s.begin_list(self.transactions.len());
        for transaction in &self.transactions {
            let transaction = ethers_core::types::Transaction::from(transaction.clone());
            s.append_raw(&transaction.rlp(), 1);
        }

        // Uncles block headers. Currently not supported
        {
            s.begin_list(0);
        }
    }
}

fn block_header_rlp<T>(block: &Block<T>, s: &mut RlpStream) {
    // Block header
    let len = 15 + (block.base_fee_per_gas.is_some() as usize);

    s.begin_list(len);
    s.append(&block.parent_hash);
    s.append(&block.uncles_hash);
    s.append(&block.author);
    s.append(&block.state_root);
    s.append(&block.transactions_root);
    s.append(&block.receipts_root);
    s.append(&block.logs_bloom);
    s.append(&block.difficulty);
    s.append(&block.number);
    s.append(&block.gas_limit);
    s.append(&block.gas_used);
    s.append(&block.timestamp);
    s.append(&block.extra_data);
    s.append(&block.mix_hash);
    s.append(&block.nonce);

    if let Some(base_fee) = block.base_fee_per_gas.as_ref() {
        s.append(base_fee);
    }
}

impl Decodable for Block<Transaction> {
    fn decode(r: &Rlp) -> Result<Self, DecoderError> {
        let header = r.at(0)?;
        let item_count = header.item_count()?;

        let mut block = Self {
            parent_hash: header.val_at(0)?,
            uncles_hash: header.val_at(1)?,
            author: header.val_at(2)?,
            state_root: header.val_at(3)?,
            transactions_root: header.val_at(4)?,
            receipts_root: header.val_at(5)?,
            logs_bloom: header.val_at(6)?,
            difficulty: header.val_at(7)?,
            number: header.val_at(8)?,
            gas_limit: header.val_at(9)?,
            gas_used: header.val_at(10)?,
            timestamp: header.val_at(11)?,
            extra_data: header.val_at::<Vec<_>>(12)?.into(),
            mix_hash: header.val_at(13)?,
            nonce: header.val_at(14)?,
            hash: Default::default(),
            total_difficulty: Default::default(),
            seal_fields: Vec::new(),
            uncles: Vec::new(),
            transactions: Vec::new(),
            size: None,
            base_fee_per_gas: if item_count > 15 {
                Some(header.val_at(15)?)
            } else {
                None
            },
        };

        let transactions = r.at(1)?;
        let transactions_count = transactions.item_count()?;
        block.transactions.reserve(transactions_count);
        for i in 0..transactions_count {
            let tx_rlp = transactions.at(i)?;

            let tx_bytes = match tx_rlp.is_data() {
                true => tx_rlp.data()?,
                false => tx_rlp.as_raw(),
            };

            let tx: ethers_core::types::Transaction = rlp::decode(tx_bytes)?;

            block.transactions.push(tx.into());
        }

        Ok(block)
    }
}

/// Calculate the hash of a block
pub fn calculate_block_hash<T>(block: &Block<T>) -> H256 {
    let mut rlp = RlpStream::new();
    block_header_rlp(block, &mut rlp);
    keccak_hash(&rlp.out())
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

    U256::from(block_size + transactions_size + size_field_size)
}

impl Storable for Block<H256> {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        codec::encode(self).into()
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        codec::decode(&bytes)
    }
}

impl SlicedStorable for Block<H256> {
    // Most blocks in tests takes less then 500 bytes.
    const CHUNK_SIZE: ChunkSize = 512;
}

impl<D, T: From<D>> From<ethers_core::types::Block<D>> for Block<T> {
    fn from(block: ethers_core::types::Block<D>) -> Self {
        Block {
            hash: block.hash.map(Into::into).unwrap_or_default(),
            parent_hash: block.parent_hash.into(),
            uncles_hash: block.uncles_hash.into(),
            author: block.author.map(Into::into).unwrap_or_default(),
            state_root: block.state_root.into(),
            transactions_root: block.transactions_root.into(),
            receipts_root: block.receipts_root.into(),
            number: block.number.map(Into::into).unwrap_or_default(),
            gas_used: block.gas_used.into(),
            gas_limit: block.gas_limit.into(),
            extra_data: block.extra_data.into(),
            logs_bloom: block.logs_bloom.map(Into::into).unwrap_or_default(),
            timestamp: block.timestamp.into(),
            difficulty: block.difficulty.into(),
            total_difficulty: block.total_difficulty.map(Into::into).unwrap_or_default(),
            seal_fields: block.seal_fields.into_iter().map(Into::into).collect(),
            uncles: block.uncles.into_iter().map(Into::into).collect(),
            transactions: block.transactions.into_iter().map(Into::into).collect(),
            size: block.size.map(Into::into),
            mix_hash: block.mix_hash.map(Into::into).unwrap_or_default(),
            nonce: block.nonce.map(Into::into).unwrap_or_default(),
            base_fee_per_gas: block.base_fee_per_gas.map(Into::into),
        }
    }
}

impl<D, T: From<D>> From<Block<D>> for ethers_core::types::Block<T> {
    fn from(block: Block<D>) -> Self {
        ethers_core::types::Block {
            hash: Some(block.hash.into()),
            parent_hash: block.parent_hash.into(),
            uncles_hash: block.uncles_hash.into(),
            author: Some(block.author.into()),
            state_root: block.state_root.into(),
            transactions_root: block.transactions_root.into(),
            receipts_root: block.receipts_root.into(),
            number: Some(block.number.into()),
            gas_used: block.gas_used.into(),
            gas_limit: block.gas_limit.into(),
            extra_data: block.extra_data.into(),
            logs_bloom: Some(block.logs_bloom.into()),
            timestamp: block.timestamp.into(),
            difficulty: block.difficulty.into(),
            total_difficulty: Some(block.total_difficulty.into()),
            seal_fields: block.seal_fields.into_iter().map(|x| x.into()).collect(),
            uncles: block.uncles.into_iter().map(Into::into).collect(),
            transactions: block.transactions.into_iter().map(Into::into).collect(),
            size: block.size.map(Into::into),
            mix_hash: Some(block.mix_hash.into()),
            nonce: Some(block.nonce.into()),
            base_fee_per_gas: block.base_fee_per_gas.map(Into::into),
            other: ethers_core::types::OtherFields::default(),
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

impl rlp::Encodable for TransactionExecutionLog {
    fn rlp_append(&self, s: &mut rlp::RlpStream) {
        s.begin_list(3);
        s.append(&self.address);
        s.append_list(&self.topics);
        s.append(&self.data.0);
    }
}

impl From<EthersLog> for TransactionExecutionLog {
    fn from(log: EthersLog) -> Self {
        Self {
            address: log.address.into(),
            topics: log.topics.into_iter().map(|h| h.into()).collect(),
            data: log.data.0.into(),
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
    fn to_bytes(&self) -> std::borrow::Cow<'_, [u8]> {
        codec::encode(self).into()
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        codec::decode(&bytes)
    }
}

impl SlicedStorable for ExeResult {
    // Most of ExeResult instances from tests encoded into less then 500 bytes.
    const CHUNK_SIZE: ChunkSize = 512;
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

    use std::str::FromStr;

    use candid::{Decode, Encode};
    use ethers_core::k256::ecdsa::SigningKey;

    use super::*;
    use crate::test_utils::read_all_files_to_json;
    use crate::transaction::{SigningMethod, StorableExecutionResult, TransactionBuilder};

    #[test]
    fn test_storable_block() {
        let mut block = Block {
            author: ethereum_types::H160::random().into(),
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
                ethereum_types::H256::from_str(&hash).unwrap(),
                calculated_block_hash.0
            );
        }
    }

    fn create_transaction(gas_price: Option<U256>, chain_id: u64) -> Transaction {
        TransactionBuilder {
            gas_price,
            signature: SigningMethod::SigningKey(&SigningKey::from_slice(&[4u8; 32]).unwrap()),
            from: &H160::from_slice(&[0u8; 20]),
            to: None,
            nonce: U256::zero(),
            value: U256::zero(),
            input: vec![],
            gas: 20u64.into(),
            chain_id,
        }
        .calculate_hash_and_build()
        .unwrap()
    }

    fn create_different_type_transaction(tx_type: Option<u64>) -> Transaction {
        let mut tx = ethers_core::types::Transaction {
            gas_price: Some(10_u64.into()),
            from: H160::from_slice(&[0u8; 20]).into(),
            to: None,
            nonce: U256::zero().into(),
            value: U256::zero().into(),
            gas: 20u64.into(),
            chain_id: Some(35514.into()),
            ..Default::default()
        };

        tx.transaction_type = tx_type.map(|v| v.into());

        match tx_type {
            Some(1) => {
                tx.access_list = Some(AccessList::default());
            }
            Some(2) => {
                tx.access_list = Some(AccessList::default());
                tx.max_fee_per_gas = Some(10_u64.into());
                tx.max_priority_fee_per_gas = Some(10_u64.into());
                tx.gas_price = None;
            }
            _ => {}
        }

        tx.hash = tx.hash();
        tx.into()
    }

    #[test]
    fn test_block_rlp_serialization_ethereum_tests() {
        let block_rlp = Bytes::from_hex_str("0xf90326f901f8a0f314e8e04cbafae8cfa65d4716c15ac762d3c77831b16086adab857c35032b1ba01dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347943535353535353535353535353535353535353535a0e0ce06a77f839ccb2c7245990c7cf98e35b9ddcaae739befe2affdda50914544a0f78dee9f184cb5bf7e23f9958782e45eef910df7265eb3e1dd2e564a99702408a01224213069d804ae95797649abe251834f702562bb6f57e2c97211d74d5a7ff9b901000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000083020000018405f5e10082a9608203e800a00000000000000000000000000000000000000000000000000000000000000000880000000000000000f90127f90124240a82a96094c305c901078781c232a2a521c2af7980f8385ee980b8c430c8d1da0000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000008000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000001ba06f7b08df2a13f13e14e2867ffc5a7c7b8d3b966ec5711a6343f17f7e5bc2bdb5a068d28b1c272316ee1492be6181cb583b7c35ac45be8043c2fef14ac17e9dfefec0").unwrap();

        let block = rlp::decode(&block_rlp.0);

        assert!(block.is_ok());

        let block: Block<Transaction> = block.unwrap();

        assert_eq!(block.transactions.len(), 1);
        assert_eq!(block.transactions[0].transaction_type, None);
    }

    #[test]
    fn test_block_rlp_serialization_roundtrip_non_legacy() {
        let block = Block::<Transaction> {
            author: H160::from_slice(&[3u8; 20]),
            number: U64::from(12u64),
            logs_bloom: Bloom(ethereum_types::Bloom::from_slice(&[4u8; 256])),
            nonce: H64::zero(),
            transactions: vec![
                create_different_type_transaction(Some(2)),
                create_different_type_transaction(Some(1)),
            ],
            mix_hash: Default::default(), // during the serialization empty value is equivalent to the default
            hash: Default::default(),
            parent_hash: H256::from_slice(&[1u8; 32]),
            uncles_hash: H256::from_slice(&[4u8; 32]),
            state_root: H256::from_slice(&[5u8; 32]),
            transactions_root: H256::from_slice(&[6u8; 32]),
            receipts_root: H256::from_slice(&[7u8; 32]),
            gas_used: U256::from(20u64),
            gas_limit: U256::from(30u64),
            extra_data: Default::default(),
            timestamp: U256::from(40u64),
            difficulty: U256::from(50u64),
            total_difficulty: Default::default(),
            seal_fields: Vec::new(),
            uncles: Vec::new(),
            size: None,
            base_fee_per_gas: None,
        };

        let rlp_data = rlp::encode(&block);
        let recovered_block: Block<Transaction> = rlp::decode(&rlp_data).unwrap();

        assert_eq!(block, recovered_block);
    }

    #[test]
    fn test_block_rlp_serialization_roundtrip_legacy_and_non_legacy() {
        let block = Block::<Transaction> {
            author: H160::from_slice(&[3u8; 20]),
            number: U64::from(12u64),
            logs_bloom: Bloom(ethereum_types::Bloom::from_slice(&[4u8; 256])),
            nonce: H64::zero(),
            transactions: vec![
                create_different_type_transaction(Some(2)),
                create_different_type_transaction(Some(1)),
                create_transaction(Some(Default::default()), 35514),
            ],
            mix_hash: Default::default(), // during the serialization empty value is equivalent to the default
            hash: Default::default(),
            parent_hash: H256::from_slice(&[1u8; 32]),
            uncles_hash: H256::from_slice(&[4u8; 32]),
            state_root: H256::from_slice(&[5u8; 32]),
            transactions_root: H256::from_slice(&[6u8; 32]),
            receipts_root: H256::from_slice(&[7u8; 32]),
            gas_used: U256::from(20u64),
            gas_limit: U256::from(30u64),
            extra_data: Default::default(),
            timestamp: U256::from(40u64),
            difficulty: U256::from(50u64),
            total_difficulty: Default::default(),
            seal_fields: Vec::new(),
            uncles: Vec::new(),
            size: None,
            base_fee_per_gas: None,
        };

        let rlp_data = rlp::encode(&block);
        let recovered_block: Block<Transaction> = rlp::decode(&rlp_data).unwrap();

        assert_eq!(block, recovered_block);
    }

    #[test]
    fn test_block_rlp_serialization_roundtrip() {
        let chain_id = 31154;
        let block = Block::<Transaction> {
            author: H160::from_slice(&[3u8; 20]),
            number: U64::from(12u64),
            logs_bloom: Bloom(ethereum_types::Bloom::from_slice(&[4u8; 256])),
            nonce: H64::zero(),
            transactions: vec![create_transaction(Some(Default::default()), chain_id)],
            mix_hash: Default::default(), // during the serialization empty value is equivalent to the default
            hash: Default::default(),
            parent_hash: H256::from_slice(&[1u8; 32]),
            uncles_hash: H256::from_slice(&[4u8; 32]),
            state_root: H256::from_slice(&[5u8; 32]),
            transactions_root: H256::from_slice(&[6u8; 32]),
            receipts_root: H256::from_slice(&[7u8; 32]),
            gas_used: U256::from(20u64),
            gas_limit: U256::from(30u64),
            extra_data: Default::default(),
            timestamp: U256::from(40u64),
            difficulty: U256::from(50u64),
            total_difficulty: Default::default(),
            seal_fields: Vec::new(),
            uncles: Vec::new(),
            size: None,
            base_fee_per_gas: None,
        };

        let rlp_data = rlp::encode(&block);
        let recovered_block: Block<Transaction> = rlp::decode(&rlp_data).unwrap();

        assert_eq!(block, recovered_block);
    }

    #[test]
    fn test_block_rlp_serialization_roundtrip_with_base_fee_per_gas() {
        let chain_id = 31154;
        let block = Block::<Transaction> {
            author: ethereum_types::H160::random().into(),
            number: U64::from(rand::random::<u64>()),
            logs_bloom: Bloom(ethereum_types::Bloom::from_slice(&[4u8; 256])),
            nonce: ethereum_types::H64::random().into(),
            transactions: vec![create_transaction(
                Some(U256::from(rand::random::<u64>())),
                chain_id,
            )],
            mix_hash: ethereum_types::H256::random().into(), // during the serialization empty value is equivalent to the default
            hash: Default::default(),
            parent_hash: ethereum_types::H256::random().into(),
            uncles_hash: ethereum_types::H256::random().into(),
            state_root: ethereum_types::H256::random().into(),
            transactions_root: ethereum_types::H256::random().into(),
            receipts_root: ethereum_types::H256::random().into(),
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

        let rlp_data = rlp::encode(&block);
        let recovered_block: Block<Transaction> = rlp::decode(&rlp_data).unwrap();

        assert_eq!(block, recovered_block);
    }

    #[test]
    fn test_storable_storable_exe_result() {
        let exe_result = StorableExecutionResult {
            exe_result: ExeResult::Success {
                gas_used: Default::default(),
                logs: Default::default(),
                logs_bloom: Default::default(),
                output: Default::default(),
            },
            transaction_hash: H256::from(ethereum_types::H256::random()),
            transaction_index: rand::random::<u64>().into(),
            block_hash: H256::from(ethereum_types::H256::random()),
            block_number: rand::random::<u64>().into(),
            from: H160::from(ethereum_types::H160::random()),
            to: Some(H160::from(ethereum_types::H160::random())),
            transaction_type: Default::default(),
        };

        let serialized = exe_result.to_bytes();
        let deserialized = StorableExecutionResult::from_bytes(serialized);

        assert_eq!(exe_result, deserialized);
    }

    #[test]
    fn test_candid_storable_exe_result() {
        let exe_result = StorableExecutionResult {
            exe_result: ExeResult::Halt {
                error: HaltError::CallTooDeep,
                gas_used: Default::default(),
            },
            transaction_hash: H256::from(ethereum_types::H256::random()),
            transaction_index: rand::random::<u64>().into(),
            block_hash: H256::from(ethereum_types::H256::random()),
            block_number: rand::random::<u64>().into(),
            from: H160::from(ethereum_types::H160::random()),
            to: Some(H160::from(ethereum_types::H160::random())),
            transaction_type: Default::default(),
        };

        let res0 = Encode!(&exe_result).unwrap();
        let res = Decode!(res0.as_slice(), StorableExecutionResult).unwrap();

        assert_eq!(exe_result, res);
    }

    #[test]
    fn test_serde_storable_exe_result() {
        let exe_result = StorableExecutionResult {
            exe_result: ExeResult::Revert {
                revert_message: Default::default(),
                gas_used: Default::default(),
                output: Default::default(),
            },
            transaction_hash: H256::from(ethereum_types::H256::random()),
            transaction_index: rand::random::<u64>().into(),
            block_hash: H256::from(ethereum_types::H256::random()),
            block_number: rand::random::<u64>().into(),
            from: H160::from(ethereum_types::H160::random()),
            to: Some(H160::from(ethereum_types::H160::random())),
            transaction_type: Default::default(),
        };

        let encoded_value = serde_json::json!(&exe_result);
        let decoded_value: StorableExecutionResult = serde_json::from_value(encoded_value).unwrap();

        assert_eq!(exe_result, decoded_value);
    }

    #[test]
    fn test_block_result() {
        let block = Block::<Transaction> {
            author: ethereum_types::H160::random().into(),
            number: U64::from(rand::random::<u64>()),
            logs_bloom: Bloom(ethereum_types::Bloom::from_slice(&[4u8; 256])),
            nonce: ethereum_types::H64::random().into(),
            transactions: vec![create_transaction(
                Some(U256::from(rand::random::<u64>())),
                1,
            )],
            mix_hash: ethereum_types::H256::random().into(),
            hash: Default::default(),
            parent_hash: ethereum_types::H256::random().into(),
            uncles_hash: ethereum_types::H256::random().into(),
            state_root: ethereum_types::H256::random().into(),
            transactions_root: ethereum_types::H256::random().into(),
            receipts_root: ethereum_types::H256::random().into(),
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
}
