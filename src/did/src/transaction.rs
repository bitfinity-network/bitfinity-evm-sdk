use std::borrow::Cow;
use std::rc::Rc;

use candid::types::{Type, TypeInner};
use candid::{CandidType, Deserialize};
use derive_more::Display;
use ethers_core::types::transaction::eip2930;
use ethers_core::types::Signature as EthersSignature;
use ic_stable_structures::{Bound, ChunkSize, SlicedStorable, Storable};
use rlp::{Decodable, DecoderError, Encodable, Rlp, RlpStream};
use serde::{Deserializer, Serialize, Serializer};
use sha2::Digest;
use sha3::Keccak256;

use super::hash::{H160, H256};
use super::integer::{U256, U64};
use crate::block::{ExeResult, TransactOut, TransactionExecutionLog};
use crate::error::EvmError;
use crate::{codec, Bytes};

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq)]
pub enum BlockNumber {
    Latest,
    Earliest,
    Pending,
    Number(U64),
}

impl BlockNumber {
    fn from_str(s: &str) -> Result<BlockNumber, String> {
        Ok(match s {
            "latest" => Self::Latest,
            "earliest" => Self::Earliest,
            "pending" => Self::Pending,
            n => BlockNumber::Number(U64::from_hex_str(n)?),
        })
    }
}

impl Serialize for BlockNumber {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            BlockNumber::Latest => serializer.serialize_str("latest"),
            BlockNumber::Earliest => serializer.serialize_str("earliest"),
            BlockNumber::Pending => serializer.serialize_str("pending"),
            BlockNumber::Number(ref n) => serializer.serialize_str(&n.to_hex_str()),
        }
    }
}

impl<'de> Deserialize<'de> for BlockNumber {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        BlockNumber::from_str(&String::deserialize(deserializer)?.to_lowercase()).map_err(serde::de::Error::custom)
    }
}

impl CandidType for BlockNumber {
    fn _ty() -> candid::types::Type {
        Type(Rc::new(TypeInner::Text))
    }

    fn idl_serialize<S>(&self, serializer: S) -> Result<(), S::Error>
    where
        S: candid::types::Serializer,
    {
        match *self {
            BlockNumber::Latest => serializer.serialize_text("latest"),
            BlockNumber::Earliest => serializer.serialize_text("earliest"),
            BlockNumber::Pending => serializer.serialize_text("pending"),
            BlockNumber::Number(ref n) => serializer.serialize_text(&format!("0x{n:x}")),
        }
    }
}

impl From<U64> for BlockNumber {
    fn from(n: U64) -> Self {
        Self::Number(n)
    }
}

impl From<u64> for BlockNumber {
    fn from(n: u64) -> Self {
        Self::Number(n.into())
    }
}

#[derive(Debug, Display, Clone, PartialEq, Eq)]
pub enum BlockID {
    BlockNumber(BlockNumber),
    BlockHash(H256),
}

impl Serialize for BlockID {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            BlockID::BlockHash(hash) => hash.serialize(serializer),
            BlockID::BlockNumber(number) => number.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for BlockID {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?.to_lowercase();
        if let Ok(hash) = H256::from_hex_str(&s) {
            return Ok(BlockID::BlockHash(hash))
        }

        Ok(BlockID::BlockNumber(BlockNumber::from_str(&s).map_err(serde::de::Error::custom)?))
    }
}

impl CandidType for BlockID {
    fn _ty() -> candid::types::Type {
        Type(Rc::new(TypeInner::Text))
    }

    fn idl_serialize<S>(&self, serializer: S) -> Result<(), S::Error>
    where
        S: candid::types::Serializer,
    {
        match self {
            BlockID::BlockHash(hash) => hash.idl_serialize(serializer),
            BlockID::BlockNumber(block_num) => block_num.idl_serialize(serializer),
        }
    }
}

/// ECDSA signature representation
#[derive(Debug, Clone, PartialEq, Eq, CandidType, Serialize, Deserialize, Default)]
pub struct Signature {
    pub v: U64,
    pub r: U256,
    pub s: U256,
}

impl From<Signature> for EthersSignature {
    fn from(value: Signature) -> Self {
        Self {
            r: value.r.into(),
            s: value.s.into(),
            v: value.v.into(),
        }
    }
}

impl From<EthersSignature> for Signature {
    fn from(value: EthersSignature) -> Self {
        Self {
            r: value.r.into(),
            s: value.s.into(),
            v: value.v.into(),
        }
    }
}

impl Signature {
    /// Upper limit for signature S field.
    /// See comment to `Signature::check_malleability()` for more details.
    pub const S_UPPER_LIMIT_HEX_STR: &str =
        "0x7FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF5D576E7357A4501DDFE92F46681B20A0";

    /// This comment copied from OpenZeppelin `ECDSA::tryRecover()` function.
    ///
    /// EIP-2 still allows signature malleability for ecrecover(). Remove this possibility and make the signature
    /// unique. Appendix F in the Ethereum Yellow paper (https://ethereum.github.io/yellowpaper/paper.pdf), defines
    /// the valid range for s in (301): 0 < s < secp256k1n ÷ 2 + 1, and for v in (302): v ∈ {27, 28}. Most
    /// signatures from current libraries generate a unique signature with an s-value in the lower half order.
    ///
    /// If your library generates malleable signatures, such as s-values in the upper range, calculate a new s-value
    /// with 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141 - s1 and flip v from 27 to 28 or
    /// vice versa. If your library also generates signatures with 0/1 for v instead 27/28, add 27 to v to accept
    /// these malleable signatures as well.
    pub fn check_malleability(s: &U256) -> Result<(), EvmError> {
        let upper_limit = U256::from_hex_str(Self::S_UPPER_LIMIT_HEX_STR)?;
        if s > &upper_limit {
            return Err(EvmError::TransactionSignature(format!(
                "S value in transaction signature should not exceed {upper_limit}"
            )));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, CandidType, Serialize, Deserialize, Default)]
pub struct Transaction {
    /// The transaction's hash
    pub hash: H256,

    /// The transaction's nonce
    pub nonce: U256,

    /// Block hash. None when pending.
    #[serde(default, rename = "blockHash")]
    pub block_hash: Option<H256>,

    /// Block number. None when pending.
    #[serde(default, rename = "blockNumber")]
    pub block_number: Option<U64>,

    /// Transaction Index. None when pending.
    #[serde(default, rename = "transactionIndex")]
    pub transaction_index: Option<U64>,

    /// Sender
    #[serde(default)]
    pub from: H160,

    /// Recipient (None when contract creation)
    #[serde(default)]
    pub to: Option<H160>,

    /// Transferred value
    pub value: U256,

    /// Gas Price, null for Type 2 transactions
    #[serde(rename = "gasPrice")]
    pub gas_price: Option<U256>,

    /// Gas amount
    pub gas: U256,

    /// Input data
    pub input: Bytes,

    /// ECDSA recovery id
    pub v: U64,

    /// ECDSA signature r
    pub r: U256,

    /// ECDSA signature s
    pub s: U256,

    // EIP2718
    /// Transaction type, Some(2) for EIP-1559 transaction,
    /// Some(1) for AccessList transaction, None for Legacy
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub transaction_type: Option<U64>,

    // EIP2930
    #[serde(
        rename = "accessList",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub access_list: Option<AccessList>,

    /// Represents the maximum tx fee that will go to the miner as part of the user's
    /// fee payment. It serves 3 purposes:
    /// 1. Compensates miners for the uncle/ommer risk + fixed costs of including transaction in a
    /// block; 2. Allows users with high opportunity costs to pay a premium to miners;
    /// 3. In times where demand exceeds the available block space (i.e. 100% full, 30mm gas),
    /// this component allows first price auctions (i.e. the pre-1559 fee model) to happen on the
    /// priority fee.
    ///
    /// More context [here](https://hackmd.io/@q8X_WM2nTfu6nuvAzqXiTQ/1559-wallets)
    #[serde(
        rename = "maxPriorityFeePerGas",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub max_priority_fee_per_gas: Option<U256>,

    /// Represents the maximum amount that a user is willing to pay for their tx (inclusive of
    /// baseFeePerGas and maxPriorityFeePerGas). The difference between maxFeePerGas and
    /// baseFeePerGas + maxPriorityFeePerGas is “refunded” to the user.
    #[serde(
        rename = "maxFeePerGas",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub max_fee_per_gas: Option<U256>,

    #[serde(rename = "chainId", default, skip_serializing_if = "Option::is_none")]
    pub chain_id: Option<U256>,
}

impl From<ethers_core::types::Transaction> for Transaction {
    fn from(tx: ethers_core::types::Transaction) -> Self {
        Self {
            hash: tx.hash.into(),
            nonce: tx.nonce.into(),
            block_hash: tx.block_hash.map(Into::into),
            block_number: tx.block_number.map(Into::into),
            transaction_index: tx.transaction_index.map(Into::into),
            from: tx.from.into(),
            to: tx.to.map(Into::into),
            value: tx.value.into(),
            gas_price: tx.gas_price.map(Into::into),
            gas: tx.gas.into(),
            input: tx.input.into(),
            v: tx.v.into(),
            r: tx.r.into(),
            s: tx.s.into(),
            transaction_type: tx.transaction_type.map(Into::into),
            access_list: tx.access_list.map(Into::into),
            max_priority_fee_per_gas: tx.max_priority_fee_per_gas.map(Into::into),
            max_fee_per_gas: tx.max_fee_per_gas.map(Into::into),
            chain_id: tx.chain_id.map(Into::into),
        }
    }
}

impl From<Transaction> for ethers_core::types::Transaction {
    fn from(tx: Transaction) -> Self {
        Self {
            hash: tx.hash.into(),
            nonce: tx.nonce.into(),
            block_hash: tx.block_hash.map(Into::into),
            block_number: tx.block_number.map(Into::into),
            transaction_index: tx.transaction_index.map(Into::into),
            from: tx.from.into(),
            to: tx.to.map(Into::into),
            value: tx.value.into(),
            gas_price: tx.gas_price.map(Into::into),
            gas: tx.gas.into(),
            input: tx.input.into(),
            v: tx.v.into(),
            r: tx.r.into(),
            s: tx.s.into(),
            transaction_type: tx.transaction_type.map(Into::into),
            access_list: tx.access_list.map(Into::into),
            max_priority_fee_per_gas: tx.max_priority_fee_per_gas.map(Into::into),
            max_fee_per_gas: tx.max_fee_per_gas.map(Into::into),
            chain_id: tx.chain_id.map(Into::into),
            other: ethers_core::types::OtherFields::default(),
        }
    }
}

impl Storable for Transaction {
    const BOUND: Bound = Bound::Unbounded;

    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        codec::encode(self).into()
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        codec::decode(&bytes)
    }
}

impl SlicedStorable for Transaction {
    // Most of test transactions takes about 250 bytes
    const CHUNK_SIZE: ic_stable_structures::ChunkSize = 256;
}

#[derive(Debug, Clone, PartialEq, Eq, CandidType, Serialize, Deserialize, Default)]
pub struct AccessListItem {
    pub address: H160,
    #[serde(default, rename = "storageKeys")]
    pub storage_keys: Vec<H256>,
}

#[derive(Clone, Serialize, Deserialize, Default, PartialEq, Eq, Debug, CandidType)]
pub struct AccessList(pub Vec<AccessListItem>);

impl From<eip2930::AccessList> for AccessList {
    fn from(access_list: eip2930::AccessList) -> Self {
        AccessList(
            access_list
                .0
                .into_iter()
                .map(|access_list| AccessListItem {
                    address: access_list.address.into(),
                    storage_keys: access_list
                        .storage_keys
                        .into_iter()
                        .map(Into::into)
                        .collect(),
                })
                .collect(),
        )
    }
}
impl From<AccessList> for eip2930::AccessList {
    fn from(access_list: AccessList) -> Self {
        eip2930::AccessList(
            access_list
                .0
                .into_iter()
                .map(|access_list| eip2930::AccessListItem {
                    address: access_list.address.into(),
                    storage_keys: access_list
                        .storage_keys
                        .into_iter()
                        .map(Into::into)
                        .collect(),
                })
                .collect(),
        )
    }
}

/// "Receipt" of an executed transaction: details of its execution.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct TransactionReceipt {
    /// Transaction hash.
    #[serde(rename = "transactionHash")]
    pub transaction_hash: H256,
    /// Index within the block.
    #[serde(rename = "transactionIndex")]
    pub transaction_index: U64,
    /// Hash of the block this transaction was included within.
    #[serde(rename = "blockHash")]
    pub block_hash: H256,
    /// Number of the block this transaction was included within.
    #[serde(rename = "blockNumber")]
    pub block_number: U64,
    /// address of the sender.
    pub from: H160,
    // address of the receiver. null when its a contract creation transaction.
    pub to: Option<H160>,
    /// Cumulative gas used within the block after this was executed.
    #[serde(rename = "cumulativeGasUsed")]
    pub cumulative_gas_used: U256,
    /// Gas used by this transaction alone.
    ///
    /// Gas used is `None` if the the client is running in light client mode.
    #[serde(rename = "gasUsed")]
    pub gas_used: Option<U256>,
    /// Contract address created, or `None` if not a deployment.
    #[serde(rename = "contractAddress")]
    pub contract_address: Option<H160>,
    /// Logs generated within this transaction.
    pub logs: Vec<TransactionReceiptLog>,
    /// Status: either 1 (success) or 0 (failure). Only present after activation of [EIP-658](https://eips.ethereum.org/EIPS/eip-658)
    pub status: Option<U64>,
    /// Transaction output data (in case when it is a contract call/creation)
    pub output: Option<Vec<u8>>,
    /// State root. Only present before activation of [EIP-658](https://eips.ethereum.org/EIPS/eip-658)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root: Option<H256>,
    /// Logs bloom
    #[serde(rename = "logsBloom")]
    pub logs_bloom: Bloom,
    /// Transaction type, Some(1) for AccessList transaction, None for Legacy
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub transaction_type: Option<U64>,
    /// The price paid post-execution by the transaction (i.e. base fee + priority fee).
    /// Both fields in 1559-style transactions are *maximums* (max fee + max priority fee), the
    /// amount that's actually paid by users can only be determined post-execution
    #[serde(
        rename = "effectiveGasPrice",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub effective_gas_price: Option<U256>,
}

impl From<StorableExecutionResult> for TransactionReceipt {
    fn from(tx_receipt: StorableExecutionResult) -> Self {
        struct ExeResultData {
            status: U64,
            gas_used: U256,
            logs: Vec<TransactionExecutionLog>,
            logs_bloom: Bloom,
            contract_address: Option<H160>,
            output: Option<Vec<u8>>,
        }

        let exe_data = match tx_receipt.exe_result {
            ExeResult::Success {
                gas_used,
                logs,
                logs_bloom,
                output,
            } => {
                let (contract_address, output) = match output {
                    TransactOut::Create(output, address) => (address, Some(output)),
                    TransactOut::Call(output) => (None, Some(output)),
                    TransactOut::None => (None, None),
                };

                ExeResultData {
                    status: U64::one(),
                    gas_used,
                    logs,
                    logs_bloom: *logs_bloom,
                    contract_address,
                    output,
                }
            }
            ExeResult::Revert {
                gas_used, output, ..
            } => ExeResultData {
                status: U64::zero(),
                gas_used,
                logs: vec![],
                logs_bloom: Bloom::zeros(),
                contract_address: None,
                output: Some(output.into()),
            },
            ExeResult::Halt { gas_used, .. } => ExeResultData {
                status: U64::zero(),
                gas_used,
                logs: vec![],
                logs_bloom: Bloom::zeros(),
                contract_address: None,
                output: None,
            },
        };

        TransactionReceipt {
            transaction_hash: tx_receipt.transaction_hash.clone(),
            transaction_index: tx_receipt.transaction_index,
            block_hash: tx_receipt.block_hash.clone(),
            block_number: tx_receipt.block_number,
            from: tx_receipt.from,
            to: tx_receipt.to,
            transaction_type: tx_receipt.transaction_type,
            gas_used: Some(exe_data.gas_used),
            logs: exe_data
                .logs
                .into_iter()
                .enumerate()
                .map(|(i, log)| TransactionReceiptLog {
                    address: log.address,
                    topics: log.topics,
                    data: log.data,
                    transaction_hash: tx_receipt.transaction_hash.clone(),
                    block_number: tx_receipt.block_number,
                    block_hash: tx_receipt.block_hash.clone(),
                    transaction_index: tx_receipt.transaction_index,
                    removed: false,
                    log_index: U256::from(i),
                })
                .collect(),
            logs_bloom: exe_data.logs_bloom,
            status: Some(exe_data.status),
            output: exe_data.output,
            contract_address: exe_data.contract_address,
            cumulative_gas_used: tx_receipt.cumulative_gas_used,
            ..Default::default()
        }
    }
}

/// TransactionReceipt Logs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, CandidType, Default)]
pub struct TransactionReceiptLog {
    /// The contract that emitted the log
    pub address: H160,

    /// Topics: Array of 0 to 4 32 Bytes of indexed log arguments.
    /// (In solidity: The first topic is the hash of the signature of the event
    /// (e.g. `Deposit(address,bytes32,uint256)`), except you declared the event
    /// with the anonymous specifier.)
    pub topics: Vec<H256>,

    /// Data
    pub data: Bytes,

    /// Transaction Hash
    #[serde(rename = "transactionHash")]
    pub transaction_hash: H256,

    /// Block Number
    #[serde(rename = "blockNumber")]
    pub block_number: U64,

    /// Block Hash
    #[serde(rename = "blockHash")]
    pub block_hash: H256,

    /// Integer of the transactions index position log was created from.
    /// None when it's a pending log.
    #[serde(rename = "transactionIndex")]
    pub transaction_index: U64,

    /// True when the log was removed, due to a chain reorganization.
    /// false if it's a valid log.
    #[serde(default)]
    pub removed: bool,

    /// Integer of the log index position in the block. None if it's a pending log.
    #[serde(rename = "logIndex")]
    pub log_index: U256,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct StorableExecutionResult {
    pub exe_result: ExeResult,
    pub transaction_hash: H256,
    pub transaction_index: U64,
    pub block_hash: H256,
    pub block_number: U64,
    pub from: H160,
    pub to: Option<H160>,
    pub transaction_type: Option<U64>,
    pub cumulative_gas_used: U256,
    pub max_fee_per_gas: Option<U256>,
    pub gas_price: Option<U256>,
    pub max_priority_fee_per_gas: Option<U256>,
}

impl Storable for StorableExecutionResult {
    const BOUND: Bound = Bound::Unbounded;

    fn to_bytes(&self) -> std::borrow::Cow<'_, [u8]> {
        codec::encode(self).into()
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        codec::decode(&bytes)
    }
}

impl SlicedStorable for StorableExecutionResult {
    const CHUNK_SIZE: ChunkSize = 512;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bloom(pub ethereum_types::Bloom);

impl Bloom {
    pub const FILTER_LENGTH_BYTES: usize = 256;

    pub fn zeros() -> Bloom {
        Bloom(ethereum_types::Bloom::zero())
    }

    pub fn to_hex_str(&self) -> String {
        format!("0x{self:x}")
    }

    pub fn from_logs<'a>(logs: impl IntoIterator<Item = &'a TransactionExecutionLog>) -> Bloom {
        let mut result = Bloom::zeros();
        let mut processor = |index, mask| {
            result.0 .0[index] |= mask;
            true
        };
        for log in logs {
            Bloom::process_log(log, &mut processor);
        }

        result
    }

    pub fn contains_log(&self, log: &TransactionExecutionLog) -> bool {
        Bloom::process_log(log, &mut |index, mask| self.0[index] & mask == mask)
    }

    pub fn contains_bloom(&self, other: &Bloom) -> bool {
        (0..Bloom::FILTER_LENGTH_BYTES).all(|i| self.0[i] & other.0[i] == other.0[i])
    }

    fn process_log(log: &TransactionExecutionLog, f: &mut impl FnMut(usize, u8) -> bool) -> bool {
        Bloom::process_data(log.address.0.as_bytes(), f)
            && log
                .topics
                .iter()
                .all(|t| Bloom::process_data(t.0.as_bytes(), f))
    }

    fn process_data(data: &[u8], f: &mut impl FnMut(usize, u8) -> bool) -> bool {
        let mut hasher = Keccak256::new();
        hasher.update(data);
        let hash = hasher.finalize();
        let hash = hash.as_slice();
        for i in [0, 2, 4] {
            let bit_index = (hash[i + 1] as usize + ((hash[i] as usize) << 8)) & 0x7FF;
            let index = Bloom::FILTER_LENGTH_BYTES - 1 - bit_index / 8;
            let mask = 1 << (bit_index % 8);
            if !f(index, mask) {
                return false;
            }
        }

        true
    }
}

impl Default for Bloom {
    fn default() -> Self {
        Bloom::zeros()
    }
}

impl Encodable for Bloom {
    fn rlp_append(&self, s: &mut RlpStream) {
        self.0.rlp_append(s);
    }
}

impl Decodable for Bloom {
    fn decode(r: &Rlp) -> Result<Self, DecoderError> {
        Ok(Bloom(ethereum_types::Bloom::decode(r)?))
    }
}

impl std::fmt::LowerHex for Bloom {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl CandidType for Bloom {
    fn _ty() -> candid::types::Type {
        Type(Rc::new(TypeInner::Text))
    }

    fn idl_serialize<S>(&self, serializer: S) -> Result<(), S::Error>
    where
        S: candid::types::Serializer,
    {
        serializer.serialize_text(&self.to_hex_str())
    }
}

impl From<ethereum_types::Bloom> for Bloom {
    fn from(bloom: ethereum_types::Bloom) -> Self {
        Bloom(bloom)
    }
}

impl From<Bloom> for ethereum_types::Bloom {
    fn from(bloom: Bloom) -> Self {
        bloom.0
    }
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use candid::{Decode, Encode};
    use ic_stable_structures::Storable;
    use rand::Rng;
    use rlp::Encodable;

    use super::*;
    use crate::test_utils::read_all_files_to_json;
    use crate::transaction::{AccessList, AccessListItem};
    use crate::BlockNumber;

    fn make_log_1() -> TransactionExecutionLog {
        TransactionExecutionLog {
            address: H160::from_hex_str("22341ae42d6dd7384bc8584e50419ea3ac75b83f").unwrap(),
            topics: vec![H256::from_hex_str(
                "04491edcd115127caedbd478e2e7895ed80c7847e903431f94f9cfa579cad47f",
            )
            .unwrap()],
            data: Default::default(),
        }
    }

    fn make_log_2() -> TransactionExecutionLog {
        TransactionExecutionLog {
            address: H160::from_hex_str("e7fb22dfef11920312e4989a3a2b81e2ebf05986").unwrap(),
            topics: vec![
                H256::from_hex_str(
                    "7f1fef85c4b037150d3675218e0cdb7cf38fea354759471e309f3354918a442f",
                )
                .unwrap(),
                H256::from_hex_str(
                    "d85629c7eaae9ea4a10234fed31bc0aeda29b2683ebe0c1882499d272621f6b6",
                )
                .unwrap(),
            ],
            data: Bytes::from_hex_str(
                "2d690516512020171c1ec870f6ff45398cc8609250326be89915fb538e7b",
            )
            .unwrap(),
        }
    }

    #[test]
    fn test_storable_transaction() {
        let tx = Transaction {
            access_list: Some(AccessList(vec![AccessListItem {
                address: ethereum_types::H160::random().into(),
                storage_keys: vec![ethereum_types::H256::random().into()],
            }])),
            ..Default::default()
        };

        let serialized = tx.to_bytes();
        let deserialized = Transaction::from_bytes(serialized);

        assert_eq!(tx, deserialized);
    }

    #[test]
    fn test_candid_encoding_transaction() {
        let tx = Transaction {
            access_list: Some(AccessList(vec![AccessListItem {
                address: ethereum_types::H160::random().into(),
                storage_keys: vec![ethereum_types::H256::random().into()],
            }])),
            ..Default::default()
        };

        let res0 = Encode!(&tx).unwrap();
        let res = Decode!(res0.as_slice(), Transaction).unwrap();
        assert_eq!(tx, res);
    }

    #[test]
    fn test_hardcoded_bloom() {
        let logs = vec![make_log_1(), make_log_2()];

        let bloom = Bloom::from_logs(&logs);
        assert_eq!(
            bloom,
            Bloom(ethereum_types::Bloom::from_str(
                "000000000000000000810000000000000000000000000000000000020000000000000000000000000000008000\
                 000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000\
                 000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000\
                 000000000000000000000000000000000000000000000000000000280000000000400000800000004000000000\
                 000000000000000000000000000000000000000000000000000000000000100000100000000000000000000000\
                 00000000001400000000000000008000000000000000000000000000000000"
            ).unwrap())
        );

        for ref log in logs {
            assert!(bloom.contains_log(log));
        }
    }

    #[test]
    fn test_bloom_combination() {
        let bloom_1 = Bloom::from_logs(&[make_log_1()]);
        let bloom_2 = Bloom::from_logs(&[make_log_2()]);
        let bloom_1_2 = Bloom::from_logs(&[make_log_1(), make_log_2()]);
        assert_eq!(bloom_1_2, Bloom(bloom_1.0 | bloom_2.0));

        assert_eq!(bloom_1_2, Bloom(bloom_2.0 | bloom_1.0));
        assert!(bloom_1_2.contains_bloom(&bloom_1_2));
        assert!(bloom_1_2.contains_bloom(&bloom_1));
        assert!(bloom_1_2.contains_bloom(&bloom_2));
        assert!(bloom_1_2.contains_bloom(&Bloom::zeros()));

        let mut bloom = Bloom::zeros();
        bloom.0 |= &bloom_1.0;
        assert_eq!(bloom, bloom_1);

        bloom.0 |= &bloom_1.0;
        assert_eq!(bloom, bloom_1);

        bloom.0 |= &bloom_2.0;
        assert_eq!(bloom, bloom_1_2);
    }

    #[test]
    fn test_rlp_encoding_bloom() {
        let mut data = [0_u8; Bloom::FILTER_LENGTH_BYTES];
        rand::thread_rng().fill(&mut data);
        let bloom = Bloom(data.into());

        let mut stream = rlp::RlpStream::new();
        bloom.rlp_append(&mut stream);
        let encoded = stream.out();
        let decoded = rlp::decode::<Bloom>(&encoded).unwrap();

        assert_eq!(bloom, decoded);
    }

    #[test]
    fn test_candid_type_bloom() {
        let mut data = [0_u8; Bloom::FILTER_LENGTH_BYTES];
        rand::thread_rng().fill(&mut data);
        let value = Bloom(data.into());

        let encoded = Encode!(&value).unwrap();
        let decoded = Decode!(&encoded, Bloom).unwrap();

        assert_eq!(value, decoded);
    }

    #[test]
    fn test_bloom_fmt_lower_hex() {
        let mut data = [0_u8; Bloom::FILTER_LENGTH_BYTES];
        rand::thread_rng().fill(&mut data);
        let value = Bloom(data.into());

        let lower_hex = value.to_hex_str();
        assert!(lower_hex.starts_with("0x"));
        assert_eq!(
            value,
            Bloom(ethereum_types::Bloom::from_str(&lower_hex).unwrap())
        );
    }

    #[test]
    fn test_bloom_serde_serialization() {
        let mut data = [0_u8; Bloom::FILTER_LENGTH_BYTES];
        rand::thread_rng().fill(&mut data);
        let value = Bloom(data.into());

        let encoded_value = serde_json::json!(&value);
        let decoded_value: Bloom = serde_json::from_value(encoded_value).unwrap();

        assert_eq!(value, decoded_value);
    }

    #[test]
    fn test_encoding_decoding_block_number() {
        let block = BlockNumber::Latest;
        let res0 = Encode!(&block).unwrap();
        let res = Decode!(res0.as_slice(), BlockNumber).unwrap();
        assert_eq!(block, res);

        let block = BlockNumber::Number(123_u64.into());
        let res0 = Encode!(&block).unwrap();
        let res = Decode!(res0.as_slice(), BlockNumber).unwrap();
        assert_eq!(block, res);

        let block = BlockNumber::Earliest;
        let res0 = Encode!(&block).unwrap();
        let res = Decode!(res0.as_slice(), BlockNumber).unwrap();
        assert_eq!(block, res);

        let block = BlockNumber::Pending;
        let res0 = Encode!(&block).unwrap();
        let res = Decode!(res0.as_slice(), BlockNumber).unwrap();
        assert_eq!(block, res);
    }

    #[test]
    fn test_encoding_decoding_block_id() {
        let block = BlockID::BlockNumber(BlockNumber::Latest);
        let res0 = Encode!(&block).unwrap();
        let res = Decode!(res0.as_slice(), BlockID).unwrap();
        assert_eq!(block, res);

        let block = BlockID::BlockNumber(BlockNumber::Number(123_u64.into()));
        let res0 = Encode!(&block).unwrap();
        let res = Decode!(res0.as_slice(), BlockID).unwrap();
        assert_eq!(block, res);

        let block = BlockID::BlockNumber(BlockNumber::Earliest);
        let res0 = Encode!(&block).unwrap();
        let res = Decode!(res0.as_slice(), BlockID).unwrap();
        assert_eq!(block, res);

        let block = BlockID::BlockNumber(BlockNumber::Pending);
        let res0 = Encode!(&block).unwrap();
        let res = Decode!(res0.as_slice(), BlockID).unwrap();
        assert_eq!(block, res);

        let block = BlockID::BlockHash(H256::from_slice(&[42; 32]));
        let res0 = Encode!(&block).unwrap();
        let res = Decode!(res0.as_slice(), BlockID).unwrap();
        assert_eq!(block, res);
    }

    #[test]
    fn test_parse_real_transactions_from_ethereum() {
        let jsons = read_all_files_to_json("./tests/resources/json/transaction");

        for (hash, value) in jsons {
            println!("Check transaction {}", hash);

            let transaction_from_value = value.get("result").unwrap().to_owned();
            let transaction: Transaction =
                serde_json::from_value(transaction_from_value.clone()).unwrap();

            let ethers_transaction: ethers_core::types::Transaction = transaction.clone().into();
            assert_eq!(
                ethereum_types::H256::from_str(&hash).unwrap(),
                ethers_transaction.hash()
            );

            let transaction_to_value = serde_json::to_value(transaction).unwrap();
            assert_eq!(transaction_from_value, transaction_to_value);

            let ethers_transaction_to_value = serde_json::to_value(ethers_transaction).unwrap();
            assert_eq!(transaction_from_value, ethers_transaction_to_value)
        }
    }

    #[test]
    fn test_from_success_call_exe_result_to_transaction_receipt() {
        let exe_result = StorableExecutionResult {
            exe_result: ExeResult::Success {
                gas_used: rand::random::<u64>().into(),
                logs: Default::default(),
                logs_bloom: Default::default(),
                output: TransactOut::Call(vec![]),
            },
            transaction_hash: H256::from(ethereum_types::H256::random()),
            transaction_index: rand::random::<u64>().into(),
            block_hash: H256::from(ethereum_types::H256::random()),
            block_number: rand::random::<u64>().into(),
            from: H160::from(ethereum_types::H160::random()),
            to: Some(H160::from(ethereum_types::H160::random())),
            transaction_type: Default::default(),
            cumulative_gas_used: rand::random::<u64>().into(),
            gas_price: Default::default(),
            max_fee_per_gas: Default::default(),
            max_priority_fee_per_gas: Default::default(),
        };

        let receipt: TransactionReceipt = exe_result.clone().into();
        assert_eq!(receipt.status, Some(U64::one()));
        assert_eq!(receipt.block_hash, exe_result.block_hash);
        assert_eq!(receipt.from, exe_result.from);
        assert_eq!(receipt.contract_address, None);
        assert_eq!(receipt.block_hash, exe_result.block_hash);
        assert_eq!(receipt.cumulative_gas_used, exe_result.cumulative_gas_used);
    }

    #[test]
    fn test_from_success_create_exe_result_to_transaction_receipt() {
        let contract_address = H160::from(ethereum_types::H160::random());
        let exe_result = StorableExecutionResult {
            exe_result: ExeResult::Success {
                gas_used: rand::random::<u64>().into(),
                logs: Default::default(),
                logs_bloom: Default::default(),
                output: TransactOut::Create(vec![1, 2], Some(contract_address.clone())),
            },
            transaction_hash: H256::from(ethereum_types::H256::random()),
            transaction_index: rand::random::<u64>().into(),
            block_hash: H256::from(ethereum_types::H256::random()),
            block_number: rand::random::<u64>().into(),
            from: H160::from(ethereum_types::H160::random()),
            to: Some(H160::from(ethereum_types::H160::random())),
            transaction_type: Default::default(),
            cumulative_gas_used: rand::random::<u64>().into(),
            gas_price: Default::default(),
            max_fee_per_gas: Default::default(),
            max_priority_fee_per_gas: Default::default(),
        };

        let receipt: TransactionReceipt = exe_result.clone().into();
        assert_eq!(receipt.status, Some(U64::one()));
        assert_eq!(receipt.block_hash, exe_result.block_hash);
        assert_eq!(receipt.from, exe_result.from);
        assert_eq!(receipt.contract_address, Some(contract_address));
        assert_eq!(receipt.block_hash, exe_result.block_hash);
        assert_eq!(receipt.output, Some(vec![1, 2]));
        assert_eq!(receipt.cumulative_gas_used, exe_result.cumulative_gas_used);
    }

    #[test]
    fn test_from_revert_exe_result_to_transaction_receipt() {
        let exe_result = StorableExecutionResult {
            exe_result: ExeResult::Revert {
                revert_message: None,
                gas_used: rand::random::<u64>().into(),
                output: vec![1, 2, 3].into(),
            },
            transaction_hash: H256::from(ethereum_types::H256::random()),
            transaction_index: rand::random::<u64>().into(),
            block_hash: H256::from(ethereum_types::H256::random()),
            block_number: rand::random::<u64>().into(),
            from: H160::from(ethereum_types::H160::random()),
            to: Some(H160::from(ethereum_types::H160::random())),
            transaction_type: Default::default(),
            cumulative_gas_used: rand::random::<u64>().into(),
            gas_price: Default::default(),
            max_fee_per_gas: Default::default(),
            max_priority_fee_per_gas: Default::default(),
        };

        let receipt: TransactionReceipt = exe_result.clone().into();
        assert_eq!(receipt.status, Some(U64::zero()));
        assert_eq!(receipt.block_hash, exe_result.block_hash);
        assert_eq!(receipt.from, exe_result.from);
        assert_eq!(receipt.contract_address, None);
        assert_eq!(receipt.block_hash, exe_result.block_hash);
        assert_eq!(receipt.output, Some(vec![1, 2, 3]));
        assert_eq!(receipt.cumulative_gas_used, exe_result.cumulative_gas_used);
    }

    #[test]
    fn test_from_halt_exe_result_to_transaction_receipt() {
        let exe_result = StorableExecutionResult {
            exe_result: ExeResult::Halt {
                gas_used: rand::random::<u64>().into(),
                error: crate::HaltError::PriorityFeeGreaterThanMaxFee,
            },
            transaction_hash: H256::from(ethereum_types::H256::random()),
            transaction_index: rand::random::<u64>().into(),
            block_hash: H256::from(ethereum_types::H256::random()),
            block_number: rand::random::<u64>().into(),
            from: H160::from(ethereum_types::H160::random()),
            to: Some(H160::from(ethereum_types::H160::random())),
            transaction_type: Default::default(),
            cumulative_gas_used: rand::random::<u64>().into(),
            gas_price: Default::default(),
            max_fee_per_gas: Default::default(),
            max_priority_fee_per_gas: Default::default(),
        };

        let receipt: TransactionReceipt = exe_result.clone().into();
        assert_eq!(receipt.status, Some(U64::zero()));
        assert_eq!(receipt.block_hash, exe_result.block_hash);
        assert_eq!(receipt.from, exe_result.from);
        assert_eq!(receipt.contract_address, None);
        assert_eq!(receipt.block_hash, exe_result.block_hash);
        assert_eq!(receipt.output, None);
        assert_eq!(receipt.cumulative_gas_used, exe_result.cumulative_gas_used);
    }

    #[test]
    fn signature_conversion_roundtrip() {
        let signature = Signature {
            r: U256::max_value(),
            s: U256::max_value() - U256::one(),
            v: U64::max_value(),
        };
        let ethers_signature = EthersSignature::from(signature.clone());
        let roundtrip_signature = Signature::from(ethers_signature);
        assert_eq!(signature, roundtrip_signature);
    }

    #[test]
    fn test_signature_malleability_check() {
        let s = U256::from_hex_str(Signature::S_UPPER_LIMIT_HEX_STR).unwrap();
        Signature::check_malleability(&s).unwrap();

        // If signature S field exceeds the limit, it should return an error.
        Signature::check_malleability(&(s + U256::one())).unwrap_err();
    }
}
