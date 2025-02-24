use std::borrow::Cow;
use std::rc::Rc;
use std::str::FromStr;
use std::sync::LazyLock;

use alloy::consensus::transaction::to_eip155_value;
use alloy::consensus::{
    SignableTransaction, Transaction as TransactionTrait, TxEip1559, TxEip2930, TxLegacy,
};
use alloy::primitives::normalize_v;
use bytes::BytesMut;
use candid::types::{Type, TypeInner};
use candid::{CandidType, Deserialize};
use derive_more::{Display, From};
use ic_stable_structures::{Bound, Storable};
use serde::{Deserializer, Serialize, Serializer};
use sha2::Digest;
use sha3::Keccak256;

use super::hash::{H160, H256};
use super::integer::{U256, U64};
use crate::block::{ExeResult, TransactOut, TransactionExecutionLog};
use crate::constant::{
    TRANSACTION_TYPE_EIP1559, TRANSACTION_TYPE_EIP2930, TRANSACTION_TYPE_LEGACY,
};
use crate::error::EvmError;
use crate::keccak::keccak_hash;
use crate::{codec, Bytes};

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq)]
pub enum BlockNumber {
    Latest,
    Earliest,
    Pending,
    Safe,
    Finalized,
    Number(U64),
}

impl BlockNumber {
    fn from_str(s: &str) -> Result<BlockNumber, String> {
        Ok(match s {
            "latest" => Self::Latest,
            "earliest" => Self::Earliest,
            "pending" => Self::Pending,
            "safe" => Self::Safe,
            "finalized" => Self::Finalized,
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
            BlockNumber::Safe => serializer.serialize_str("safe"),
            BlockNumber::Finalized => serializer.serialize_str("finalized"),
            BlockNumber::Number(ref n) => serializer.serialize_str(&n.to_hex_str()),
        }
    }
}

impl<'de> Deserialize<'de> for BlockNumber {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        BlockNumber::from_str(&String::deserialize(deserializer)?.to_lowercase())
            .map_err(serde::de::Error::custom)
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
            BlockNumber::Safe => serializer.serialize_text("safe"),
            BlockNumber::Finalized => serializer.serialize_text("finalized"),
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

#[derive(Debug, Display, Clone, PartialEq, Eq, From)]
pub enum BlockId {
    BlockNumber(BlockNumber),
    BlockHash(H256),
}

impl Serialize for BlockId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            BlockId::BlockHash(hash) => hash.serialize(serializer),
            BlockId::BlockNumber(number) => number.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for BlockId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?.to_lowercase();
        if let Ok(hash) = H256::from_hex_str(&s) {
            return Ok(BlockId::BlockHash(hash));
        }

        Ok(BlockId::BlockNumber(
            BlockNumber::from_str(&s).map_err(serde::de::Error::custom)?,
        ))
    }
}

impl CandidType for BlockId {
    fn _ty() -> candid::types::Type {
        Type(Rc::new(TypeInner::Text))
    }

    fn idl_serialize<S>(&self, serializer: S) -> Result<(), S::Error>
    where
        S: candid::types::Serializer,
    {
        match self {
            BlockId::BlockHash(hash) => hash.idl_serialize(serializer),
            BlockId::BlockNumber(block_num) => block_num.idl_serialize(serializer),
        }
    }
}

impl From<U64> for BlockId {
    fn from(n: U64) -> Self {
        Self::BlockNumber(n.into())
    }
}

impl From<u64> for BlockId {
    fn from(n: u64) -> Self {
        Self::BlockNumber(n.into())
    }
}

/// ECDSA signature representation
#[derive(Debug, Clone, PartialEq, Eq, CandidType, Serialize, Deserialize, Default)]
pub struct Signature {
    pub y_parity: Parity,
    pub r: U256,
    pub s: U256,
}

/// Parity of the Y coordinate of the public key
#[derive(Debug, Clone, PartialEq, Eq, CandidType, Serialize, Deserialize, Default)]
pub enum Parity {
    #[default]
    Even,
    Odd,
}

impl Parity {
    /// Returns the parity bool value
    pub fn as_bool(&self) -> bool {
        match self {
            Parity::Even => false,
            Parity::Odd => true,
        }
    }

    /// Returns a Parity from a y_parity_is_odd bool value.
    pub fn from_y_parity_is_odd(y_parity_is_odd: bool) -> Self {
        if y_parity_is_odd {
            Parity::Odd
        } else {
            Parity::Even
        }
    }
}

/// Transaction type and Chain id data required to generate the Signature V value for EIP-155
pub enum TxChainInfo {
    /// A legacy transation with (EIP-155) or without (not EIP-155) chain id
    LegacyTx { chain_id: Option<u64> },
    /// Other transaction types
    OtherTx,
}

impl Signature {
    /// Creates a new signature from the given r, s and v values.
    pub fn new_from_rsv(r: U256, s: U256, v: u64) -> Result<Self, EvmError> {
        let y_parity_is_odd =
            normalize_v(v).ok_or_else(|| EvmError::InvalidSignatureParity(format!("{}", v)))?;
        Ok(Self {
            r,
            s,
            y_parity: Parity::from_y_parity_is_odd(y_parity_is_odd),
        })
    }

    /// Recovers an [`Address`] from this signature and the given prehashed message.
    /// e.g.: signature.recover_from(&tx.signature_hash())
    pub fn recover_from(&self, signature_hash: &H256) -> Result<H160, EvmError> {
        let primitive_signature = alloy::primitives::PrimitiveSignature::from(self);
        let recovered_from = primitive_signature
            .recover_address_from_prehash(&signature_hash.0)
            .map_err(|err| EvmError::SignatureError(format!("{err:?}")))?;
        Ok(recovered_from.into())
    }

    /// Calculates the V signature value from the signature and the [TxChainInfo].
    /// The V value is calculated as follows:
    /// - For legacy transactions, the V value is calculated based on the EIP-155
    /// - For other transactions, the V value is the Y parity value
    pub fn v(&self, info: TxChainInfo) -> u64 {
        match info {
            TxChainInfo::LegacyTx { chain_id } => {
                to_eip155_value(self.y_parity.as_bool(), chain_id) as u64
            }
            TxChainInfo::OtherTx => self.y_parity.as_bool() as u64,
        }
    }
}

impl From<Signature> for alloy::primitives::PrimitiveSignature {
    fn from(value: Signature) -> Self {
        alloy::primitives::PrimitiveSignature::new(value.r.0, value.s.0, value.y_parity.as_bool())
    }
}

impl From<&Signature> for alloy::primitives::PrimitiveSignature {
    fn from(value: &Signature) -> Self {
        alloy::primitives::PrimitiveSignature::new(value.r.0, value.s.0, value.y_parity.as_bool())
    }
}

impl From<alloy::primitives::PrimitiveSignature> for Signature {
    fn from(value: alloy::primitives::PrimitiveSignature) -> Self {
        Self {
            r: U256(value.r()),
            s: U256(value.s()),
            y_parity: Parity::from_y_parity_is_odd(value.v()),
        }
    }
}

/// Upper limit for signature S field.
/// See comment to `Signature::check_malleability()` for more details.
pub const SIGNATURE_S_UPPER_LIMIT_HEX_STR: &str =
    "0x7FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF5D576E7357A4501DDFE92F46681B20A0";

/// Upper limit for signature S field.
/// See comment to `Signature::check_malleability()` for more details.
pub static SIGNATURE_S_UPPER_LIMIT: LazyLock<U256> = LazyLock::new(|| {
    U256::from_hex_str(SIGNATURE_S_UPPER_LIMIT_HEX_STR).expect("Invalid S upper limit")
});

impl Signature {
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
        if s > &SIGNATURE_S_UPPER_LIMIT {
            return Err(EvmError::TransactionSignature(format!(
                "S value in transaction signature should not exceed {}",
                SIGNATURE_S_UPPER_LIMIT_HEX_STR
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
    ///    block;
    /// 2. Allows users with high opportunity costs to pay a premium to miners;
    /// 3. In times where demand exceeds the available block space (i.e. 100% full, 30mm gas),
    ///    this component allows first price auctions (i.e. the pre-1559 fee model) to happen on the
    ///    priority fee.
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

impl From<alloy::consensus::TxEnvelope> for Transaction {
    fn from(tx: alloy::consensus::TxEnvelope) -> Self {
        let tx = alloy::rpc::types::Transaction {
            inner: tx,
            block_hash: None,
            block_number: None,
            transaction_index: None,
            effective_gas_price: None,
            from: Default::default(),
        };
        tx.into()
    }
}

impl TryFrom<Transaction> for alloy::consensus::TxEnvelope {
    type Error = EvmError;
    fn try_from(value: Transaction) -> Result<Self, Self::Error> {
        let tx = alloy::rpc::types::Transaction::try_from(value)?;
        Ok(tx.inner)
    }
}

impl From<alloy::rpc::types::Transaction> for Transaction {
    fn from(tx: alloy::rpc::types::Transaction) -> Self {
        let signature = tx.inner.signature();
        let signature = Signature::from(*signature);

        match tx.inner {
            alloy::consensus::TxEnvelope::Legacy(signed) => {
                let inner_tx = signed.tx();
                let chain_id = inner_tx.chain_id();
                Self {
                    hash: (*signed.hash()).into(),
                    nonce: inner_tx.nonce.into(),
                    to: inner_tx.to().map(Into::into),
                    value: inner_tx.value.into(),
                    gas_price: Some(inner_tx.gas_price.into()),
                    gas: inner_tx.gas_limit.into(),
                    input: inner_tx.input.clone().into(),
                    chain_id: inner_tx.chain_id.map(Into::into),
                    access_list: None,
                    max_priority_fee_per_gas: None,
                    max_fee_per_gas: None,
                    block_hash: tx.block_hash.map(Into::into),
                    block_number: tx.block_number.map(Into::into),
                    transaction_index: tx.transaction_index.map(Into::into),
                    from: tx.from.into(),
                    v: signature.v(TxChainInfo::LegacyTx { chain_id }).into(),
                    r: signature.r,
                    s: signature.s,
                    transaction_type: Some(TRANSACTION_TYPE_LEGACY.into()),
                }
            }
            alloy::consensus::TxEnvelope::Eip2930(signed) => {
                let inner_tx = signed.tx();
                Self {
                    hash: (*signed.hash()).into(),
                    nonce: inner_tx.nonce.into(),
                    to: inner_tx.to().map(Into::into),
                    value: inner_tx.value.into(),
                    gas_price: Some(inner_tx.gas_price.into()),
                    gas: inner_tx.gas_limit.into(),
                    input: inner_tx.input.clone().into(),
                    chain_id: Some(inner_tx.chain_id.into()),
                    access_list: Some(inner_tx.access_list.clone().into()),
                    max_priority_fee_per_gas: None,
                    max_fee_per_gas: None,
                    block_hash: tx.block_hash.map(Into::into),
                    block_number: tx.block_number.map(Into::into),
                    transaction_index: tx.transaction_index.map(Into::into),
                    from: tx.from.into(),
                    v: signature.v(TxChainInfo::OtherTx).into(),
                    r: signature.r,
                    s: signature.s,
                    transaction_type: Some(TRANSACTION_TYPE_EIP2930.into()),
                }
            }
            alloy::consensus::TxEnvelope::Eip1559(signed) => {
                let inner_tx = signed.tx();
                Self {
                    hash: (*signed.hash()).into(),
                    nonce: inner_tx.nonce.into(),
                    to: inner_tx.to().map(Into::into),
                    value: inner_tx.value.into(),
                    gas_price: None,
                    gas: inner_tx.gas_limit.into(),
                    input: inner_tx.input.clone().into(),
                    chain_id: Some(inner_tx.chain_id.into()),
                    access_list: Some(inner_tx.access_list.clone().into()),
                    max_priority_fee_per_gas: Some(inner_tx.max_priority_fee_per_gas.into()),
                    max_fee_per_gas: Some(inner_tx.max_fee_per_gas.into()),
                    block_hash: tx.block_hash.map(Into::into),
                    block_number: tx.block_number.map(Into::into),
                    transaction_index: tx.transaction_index.map(Into::into),
                    from: tx.from.into(),
                    v: signature.v(TxChainInfo::OtherTx).into(),
                    r: signature.r,
                    s: signature.s,
                    transaction_type: Some(TRANSACTION_TYPE_EIP1559.into()),
                }
            }

            _ => {
                panic!("Unsupported transaction type");
            }
        }
    }
}

impl TryFrom<Transaction> for alloy::rpc::types::Transaction {
    type Error = EvmError;

    fn try_from(tx: Transaction) -> Result<Self, EvmError> {
        let signature = Signature::new_from_rsv(tx.r, tx.s, tx.v.as_u64())?;

        let signature = alloy::primitives::PrimitiveSignature::from(signature);

        let tx_type = tx.transaction_type.unwrap_or_default().0.to::<u64>();
        match tx_type {
            TRANSACTION_TYPE_LEGACY => Ok(alloy::rpc::types::Transaction {
                inner: TxLegacy {
                    nonce: tx.nonce.0.to(),
                    gas_price: tx.gas_price.map(|v| v.0.to()).unwrap_or_default(),
                    gas_limit: tx.gas.0.to(),
                    to: tx.to.map(|v| v.0).into(),
                    value: tx.value.into(),
                    input: tx.input.into(),
                    chain_id: tx.chain_id.map(|v| v.0.to()),
                }
                .into_signed(signature)
                .into(),
                block_hash: tx.block_hash.map(Into::into),
                block_number: tx.block_number.map(Into::into),
                transaction_index: tx.transaction_index.map(Into::into),
                effective_gas_price: None,
                from: tx.from.into(),
            }),
            TRANSACTION_TYPE_EIP2930 => Ok(alloy::rpc::types::Transaction {
                inner: TxEip2930 {
                    nonce: tx.nonce.0.to(),
                    gas_price: tx.gas_price.map(|v| v.0.to()).unwrap_or_default(),
                    gas_limit: tx.gas.0.to(),
                    to: tx.to.map(|v| v.0).into(),
                    value: tx.value.into(),
                    input: tx.input.into(),
                    chain_id: tx.chain_id.map(|v| v.0.to()).unwrap_or_default(),
                    access_list: tx.access_list.map(Into::into).unwrap_or_default(),
                }
                .into_signed(signature)
                .into(),
                block_hash: tx.block_hash.map(Into::into),
                block_number: tx.block_number.map(Into::into),
                transaction_index: tx.transaction_index.map(Into::into),
                effective_gas_price: None,
                from: tx.from.into(),
            }),
            TRANSACTION_TYPE_EIP1559 => Ok(alloy::rpc::types::Transaction {
                inner: TxEip1559 {
                    nonce: tx.nonce.0.to(),
                    gas_limit: tx.gas.0.to(),
                    to: tx.to.map(|v| v.0).into(),
                    value: tx.value.into(),
                    input: tx.input.into(),
                    chain_id: tx.chain_id.map(|v| v.0.to()).unwrap_or_default(),
                    max_fee_per_gas: tx.max_fee_per_gas.map(|v| v.0.to()).unwrap_or_default(),
                    max_priority_fee_per_gas: tx
                        .max_priority_fee_per_gas
                        .map(|v| v.0.to())
                        .unwrap_or_default(),
                    access_list: tx.access_list.map(Into::into).unwrap_or_default(),
                }
                .into_signed(signature)
                .into(),
                block_hash: tx.block_hash.map(Into::into),
                block_number: tx.block_number.map(Into::into),
                transaction_index: tx.transaction_index.map(Into::into),
                effective_gas_price: None,
                from: tx.from.into(),
            }),
            _ => {
                panic!("Unsupported transaction type: {}", tx_type);
            }
        }
    }
}

impl Transaction {
    /// RLP encodes the transaction and recalculates the hash.
    /// It does not modify the transaction itself.
    /// It returns the calculated hash and the RLP encoded bytes.
    pub fn slow_hash(&self) -> Result<(H256, bytes::Bytes), EvmError> {
        let encoded = self.rlp_encoded_2718()?;
        Ok((keccak_hash(&encoded), encoded))
    }

    /// Encode the transaction according to [EIP-2718] rules. First a 1-byte
    /// type flag in the range 0x0-0x7f, then the body of the transaction.
    pub fn rlp_encoded_2718(&self) -> Result<bytes::Bytes, EvmError> {
        use alloy::eips::eip2718::Encodable2718;
        let alloy_transaction = alloy::consensus::TxEnvelope::try_from(self.clone())?;
        let mut buffer = BytesMut::new();
        alloy_transaction.encode_2718(&mut buffer);
        Ok(buffer.freeze())
    }

    /// Decode the transaction according to [EIP-2718] rules.
    pub fn from_rlp_2718(bytes: &mut &[u8]) -> Result<Self, EvmError> {
        use alloy::eips::eip2718::Decodable2718;
        alloy::consensus::TxEnvelope::decode_2718(bytes)
            .map(Into::into)
            .map_err(Into::into)
    }
}

impl Storable for Transaction {
    const BOUND: Bound = Bound::Unbounded;

    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        codec::encode(self).into()
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        let mut tx: Self = codec::decode(&bytes);
        tx.transaction_type = tx.transaction_type.or(Some(U64::zero()));
        tx
    }
}

#[derive(Debug, Clone, PartialEq, Eq, CandidType, Serialize, Deserialize, Default)]
pub struct AccessListItem {
    pub address: H160,
    #[serde(default, rename = "storageKeys")]
    pub storage_keys: Vec<H256>,
}

#[derive(Clone, Serialize, Deserialize, Default, PartialEq, Eq, Debug, CandidType)]
pub struct AccessList(pub Vec<AccessListItem>);

impl From<alloy::rpc::types::AccessList> for AccessList {
    fn from(access_list: alloy::rpc::types::AccessList) -> Self {
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
impl From<AccessList> for alloy::rpc::types::AccessList {
    fn from(access_list: AccessList) -> Self {
        alloy::rpc::types::AccessList(
            access_list
                .0
                .into_iter()
                .map(|access_list| alloy::rpc::types::AccessListItem {
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
                    status: U64::from(1u64),
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
            transaction_type: Some(tx_receipt.transaction_type.unwrap_or_default()),
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
                    log_index: U256::from(i as u64),
                })
                .collect(),
            logs_bloom: exe_data.logs_bloom,
            status: Some(exe_data.status),
            output: exe_data.output,
            contract_address: exe_data.contract_address,
            cumulative_gas_used: tx_receipt.cumulative_gas_used,
            root: None,
            effective_gas_price: tx_receipt.gas_price,
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
    /// The effective gas price paid by the transaction
    pub gas_price: Option<U256>,
    pub max_priority_fee_per_gas: Option<U256>,
    pub timestamp: u64,
}

impl Storable for StorableExecutionResult {
    const BOUND: Bound = Bound::Unbounded;

    fn to_bytes(&self) -> std::borrow::Cow<'_, [u8]> {
        codec::encode(self).into()
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        let mut ser: Self = codec::decode(&bytes);
        ser.transaction_type = ser.transaction_type.or(Some(U64::zero()));
        ser
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Bloom(pub alloy::primitives::Bloom);

impl<'de> serde::Deserialize<'de> for Bloom {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Ok(Bloom::from_hex_str(&s).unwrap())
    }
}

impl Bloom {
    pub const FILTER_LENGTH_BYTES: usize = 256;

    pub fn zeros() -> Bloom {
        Bloom(alloy::primitives::Bloom::ZERO)
    }

    pub fn to_hex_str(&self) -> String {
        format!("0x{self:x}")
    }

    pub fn from_hex_str(s: &str) -> Result<Self, String> {
        alloy::primitives::Bloom::from_str(s)
            .map_err(|e| e.to_string())
            .map(Into::into)
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

    pub fn from_slice(slice: &[u8]) -> Bloom {
        Bloom(alloy::primitives::Bloom::from_slice(slice))
    }

    pub fn contains_log(&self, log: &TransactionExecutionLog) -> bool {
        Bloom::process_log(log, &mut |index, mask| self.0[index] & mask == mask)
    }

    pub fn contains_bloom(&self, other: &Bloom) -> bool {
        (0..Bloom::FILTER_LENGTH_BYTES).all(|i| self.0[i] & other.0[i] == other.0[i])
    }

    fn process_log(log: &TransactionExecutionLog, f: &mut impl FnMut(usize, u8) -> bool) -> bool {
        Bloom::process_data(log.address.0.as_slice(), f)
            && log
                .topics
                .iter()
                .all(|t| Bloom::process_data(t.0.as_slice(), f))
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

impl alloy::rlp::Encodable for Bloom {
    fn encode(&self, out: &mut dyn bytes::BufMut) {
        self.0.encode(out);
    }
}

impl alloy::rlp::Decodable for Bloom {
    fn decode(buf: &mut &[u8]) -> alloy::rlp::Result<Self> {
        Ok(Self(alloy::primitives::Bloom::decode(buf)?))
    }
}

impl std::fmt::LowerHex for Bloom {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::LowerHex::fmt(&self.0, f)
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

impl From<alloy::primitives::Bloom> for Bloom {
    fn from(bloom: alloy::primitives::Bloom) -> Self {
        Bloom(bloom)
    }
}

impl From<Bloom> for alloy::primitives::Bloom {
    fn from(bloom: Bloom) -> Self {
        bloom.0
    }
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use candid::{Decode, Encode};
    use ic_stable_structures::Storable;
    use rand::{random, Rng};

    use super::*;
    use crate::test_utils::{read_all_files_to_json, test_candid_roundtrip, test_json_roundtrip};
    use crate::transaction::{AccessList, AccessListItem};
    use crate::{BlockNumber, HaltError};

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
                address: alloy::primitives::Address::random().into(),
                storage_keys: vec![alloy::primitives::B256::random().into()],
            }])),
            transaction_type: Some(1u64.into()),
            ..Default::default()
        };

        let serialized = tx.to_bytes();
        let deserialized = Transaction::from_bytes(serialized);

        assert_eq!(tx, deserialized);
    }

    #[test]
    fn test_storable_transaction_without_tx_type() {
        let mut tx = Transaction {
            access_list: Some(AccessList(vec![AccessListItem {
                address: alloy::primitives::Address::random().into(),
                storage_keys: vec![alloy::primitives::B256::random().into()],
            }])),
            transaction_type: None,
            ..Default::default()
        };

        let serialized = tx.to_bytes();
        let deserialized = Transaction::from_bytes(serialized);

        // Transaction type should be Some(0) after deserialization
        tx.transaction_type = Some(U64::zero());
        assert_eq!(tx, deserialized);
    }

    #[test]
    fn test_candid_encoding_transaction() {
        let tx = Transaction {
            access_list: Some(AccessList(vec![AccessListItem {
                address: alloy::primitives::Address::random().into(),
                storage_keys: vec![alloy::primitives::B256::random().into()],
            }])),
            ..Default::default()
        };

        let res0 = Encode!(&tx).unwrap();
        let res = Decode!(res0.as_slice(), Transaction).unwrap();
        assert_eq!(tx, res);
    }

    #[test]
    fn test_storable_storable_execution_result() {
        let exe_result = StorableExecutionResult {
            exe_result: ExeResult::Revert {
                revert_message: None,
                gas_used: rand::random::<u64>().into(),
                output: vec![1, 2, 3].into(),
            },
            transaction_hash: H256::from(alloy::primitives::B256::random()),
            transaction_index: rand::random::<u64>().into(),
            block_hash: H256::from(alloy::primitives::B256::random()),
            block_number: rand::random::<u64>().into(),
            from: H160::from(alloy::primitives::Address::random()),
            to: Some(H160::from(alloy::primitives::Address::random())),
            transaction_type: Some(rand::random::<u64>().into()),
            cumulative_gas_used: rand::random::<u64>().into(),
            gas_price: Some(rand::random::<u64>().into()),
            max_fee_per_gas: Default::default(),
            max_priority_fee_per_gas: Default::default(),
            timestamp: 0,
        };

        let serialized = exe_result.to_bytes();
        let deserialized = StorableExecutionResult::from_bytes(serialized);

        assert_eq!(exe_result, deserialized);
    }

    #[test]
    fn test_storable_storable_execution_result_without_tx_type() {
        let mut exe_result = StorableExecutionResult {
            exe_result: ExeResult::Revert {
                revert_message: None,
                gas_used: rand::random::<u64>().into(),
                output: vec![1, 2, 3].into(),
            },
            transaction_hash: H256::from(alloy::primitives::B256::random()),
            transaction_index: rand::random::<u64>().into(),
            block_hash: H256::from(alloy::primitives::B256::random()),
            block_number: rand::random::<u64>().into(),
            from: H160::from(alloy::primitives::Address::random()),
            to: Some(H160::from(alloy::primitives::Address::random())),
            transaction_type: None,
            cumulative_gas_used: rand::random::<u64>().into(),
            gas_price: Some(rand::random::<u64>().into()),
            max_fee_per_gas: Default::default(),
            max_priority_fee_per_gas: Default::default(),
            timestamp: 0,
        };

        let serialized = exe_result.to_bytes();
        let deserialized = StorableExecutionResult::from_bytes(serialized);

        // Transaction type should be Some(0) after deserialization
        exe_result.transaction_type = Some(U64::zero());
        assert_eq!(exe_result, deserialized);
    }

    #[test]
    fn test_candid_storable_exe_result() {
        let exe_result = StorableExecutionResult {
            exe_result: ExeResult::Halt {
                error: HaltError::CallTooDeep,
                gas_used: Default::default(),
            },
            transaction_hash: H256::from(alloy::primitives::B256::random()),
            transaction_index: rand::random::<u64>().into(),
            block_hash: H256::from(alloy::primitives::B256::random()),
            block_number: rand::random::<u64>().into(),
            from: H160::from(alloy::primitives::Address::random()),
            to: Some(H160::from(alloy::primitives::Address::random())),
            transaction_type: Default::default(),
            cumulative_gas_used: rand::random::<u64>().into(),
            gas_price: Default::default(),
            max_fee_per_gas: Default::default(),
            max_priority_fee_per_gas: Default::default(),
            timestamp: 0,
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
            transaction_hash: H256::from(alloy::primitives::B256::random()),
            transaction_index: rand::random::<u64>().into(),
            block_hash: H256::from(alloy::primitives::B256::random()),
            block_number: rand::random::<u64>().into(),
            from: H160::from(alloy::primitives::Address::random()),
            to: Some(H160::from(alloy::primitives::Address::random())),
            transaction_type: Default::default(),
            cumulative_gas_used: rand::random::<u64>().into(),
            gas_price: Default::default(),
            max_fee_per_gas: Default::default(),
            max_priority_fee_per_gas: Default::default(),
            timestamp: 0,
        };

        let encoded_value = serde_json::json!(&exe_result);
        let decoded_value: StorableExecutionResult = serde_json::from_value(encoded_value).unwrap();

        assert_eq!(exe_result, decoded_value);
    }

    #[test]
    fn test_hardcoded_bloom() {
        let logs = vec![make_log_1(), make_log_2()];

        let bloom = Bloom::from_logs(&logs);
        assert_eq!(
            bloom,
            Bloom(alloy::primitives::Bloom::from_str(
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
        bloom.0 |= bloom_1.0;
        assert_eq!(bloom, bloom_1);

        bloom.0 |= bloom_1.0;
        assert_eq!(bloom, bloom_1);

        bloom.0 |= bloom_2.0;
        assert_eq!(bloom, bloom_1_2);
    }

    #[test]
    fn test_rlp_encoding_bloom() {
        let mut data = [0_u8; Bloom::FILTER_LENGTH_BYTES];
        rand::thread_rng().fill(&mut data);
        let bloom = Bloom(data.into());

        let encoded = alloy::rlp::encode(&bloom);
        let decoded = alloy::rlp::decode_exact::<Bloom>(&encoded).unwrap();

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
            Bloom(alloy::primitives::Bloom::from_str(&lower_hex).unwrap())
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
    fn test_block_number_roundtrip() {
        let block = BlockNumber::Latest;
        test_json_roundtrip(&block);
        test_candid_roundtrip(&block);

        let block = BlockNumber::Number(123_u64.into());
        test_json_roundtrip(&block);
        test_candid_roundtrip(&block);

        let block = BlockNumber::Earliest;
        test_json_roundtrip(&block);
        test_candid_roundtrip(&block);

        let block = BlockNumber::Pending;
        test_json_roundtrip(&block);
        test_candid_roundtrip(&block);

        let block = BlockNumber::Safe;
        test_json_roundtrip(&block);
        test_candid_roundtrip(&block);

        let block = BlockNumber::Finalized;
        test_json_roundtrip(&block);
        test_candid_roundtrip(&block);
    }

    #[test]
    fn test_encoding_decoding_block_id() {
        let block = BlockId::BlockNumber(BlockNumber::Latest);
        test_json_roundtrip(&block);
        test_candid_roundtrip(&block);

        let block = BlockId::BlockNumber(BlockNumber::Number(123_u64.into()));
        test_json_roundtrip(&block);
        test_candid_roundtrip(&block);

        let block = BlockId::BlockNumber(BlockNumber::Earliest);
        test_json_roundtrip(&block);
        test_candid_roundtrip(&block);

        let block = BlockId::BlockNumber(BlockNumber::Pending);
        test_json_roundtrip(&block);
        test_candid_roundtrip(&block);

        let block = BlockId::BlockNumber(BlockNumber::Safe);
        test_json_roundtrip(&block);
        test_candid_roundtrip(&block);

        let block = BlockId::BlockNumber(BlockNumber::Finalized);
        test_json_roundtrip(&block);
        test_candid_roundtrip(&block);

        let block = BlockId::BlockHash(H256::from_slice(&[42; 32]));
        test_json_roundtrip(&block);
        test_candid_roundtrip(&block);
    }

    #[test]
    fn test_parse_real_transactions_from_ethereum() {
        let jsons = read_all_files_to_json("./tests/resources/json/transaction");

        for (hash, value) in jsons {
            println!("Check transaction {}", hash);

            let transaction_from_value = value.get("result").unwrap().to_owned();
            let transaction: Transaction =
                serde_json::from_value(transaction_from_value.clone()).unwrap();

            assert_eq!(
                alloy::primitives::B256::from_str(&hash).unwrap(),
                transaction.slow_hash().unwrap().0 .0
            );

            // from/to rlp
            {
                let (hash, rlp) = transaction.slow_hash().unwrap();
                let tx_from_rlp = Transaction::from_rlp_2718(&mut rlp.as_ref()).unwrap();

                // rlp decoded TX should have the hash set
                assert_eq!(hash, tx_from_rlp.hash);

                let (re_hash, re_rlp) = tx_from_rlp.slow_hash().unwrap();
                assert_eq!(hash, re_hash);
                assert_eq!(rlp, re_rlp);

                assert_eq!(transaction.s, tx_from_rlp.s);
                assert_eq!(transaction.r, tx_from_rlp.r);
                assert_eq!(transaction.v, tx_from_rlp.v);
            }

            let transaction_to_value = serde_json::to_value(transaction).unwrap();
            assert_eq!(transaction_from_value, transaction_to_value);
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
            transaction_hash: H256::from(alloy::primitives::B256::random()),
            transaction_index: rand::random::<u64>().into(),
            block_hash: H256::from(alloy::primitives::B256::random()),
            block_number: rand::random::<u64>().into(),
            from: H160::from(alloy::primitives::Address::random()),
            to: Some(H160::from(alloy::primitives::Address::random())),
            transaction_type: Some(rand::random::<u64>().into()),
            cumulative_gas_used: rand::random::<u64>().into(),
            gas_price: Some(rand::random::<u64>().into()),
            max_fee_per_gas: Default::default(),
            max_priority_fee_per_gas: Default::default(),
            timestamp: 0,
        };

        let receipt: TransactionReceipt = exe_result.clone().into();
        assert_eq!(receipt.status, Some(U64::from(1u64)));
        assert_eq!(receipt.block_hash, exe_result.block_hash);
        assert_eq!(receipt.from, exe_result.from);
        assert_eq!(receipt.contract_address, None);
        assert_eq!(receipt.block_hash, exe_result.block_hash);
        assert_eq!(receipt.cumulative_gas_used, exe_result.cumulative_gas_used);
        assert_eq!(receipt.effective_gas_price, exe_result.gas_price);
        assert_eq!(receipt.transaction_type, exe_result.transaction_type);
    }

    #[test]
    fn test_from_success_create_exe_result_to_transaction_receipt() {
        let contract_address = H160::from(alloy::primitives::Address::random());
        let exe_result = StorableExecutionResult {
            exe_result: ExeResult::Success {
                gas_used: rand::random::<u64>().into(),
                logs: Default::default(),
                logs_bloom: Default::default(),
                output: TransactOut::Create(vec![1, 2], Some(contract_address.clone())),
            },
            transaction_hash: H256::from(alloy::primitives::B256::random()),
            transaction_index: rand::random::<u64>().into(),
            block_hash: H256::from(alloy::primitives::B256::random()),
            block_number: rand::random::<u64>().into(),
            from: H160::from(alloy::primitives::Address::random()),
            to: Some(H160::from(alloy::primitives::Address::random())),
            transaction_type: Default::default(),
            cumulative_gas_used: rand::random::<u64>().into(),
            gas_price: Default::default(),
            max_fee_per_gas: Default::default(),
            max_priority_fee_per_gas: Default::default(),
            timestamp: 0,
        };

        let receipt: TransactionReceipt = exe_result.clone().into();
        assert_eq!(receipt.status, Some(U64::from(1u64)));
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
            transaction_hash: H256::from(alloy::primitives::B256::random()),
            transaction_index: rand::random::<u64>().into(),
            block_hash: H256::from(alloy::primitives::B256::random()),
            block_number: rand::random::<u64>().into(),
            from: H160::from(alloy::primitives::Address::random()),
            to: Some(H160::from(alloy::primitives::Address::random())),
            transaction_type: Default::default(),
            cumulative_gas_used: rand::random::<u64>().into(),
            gas_price: Default::default(),
            max_fee_per_gas: Default::default(),
            max_priority_fee_per_gas: Default::default(),
            timestamp: 0,
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
            transaction_hash: H256::from(alloy::primitives::B256::random()),
            transaction_index: rand::random::<u64>().into(),
            block_hash: H256::from(alloy::primitives::B256::random()),
            block_number: rand::random::<u64>().into(),
            from: H160::from(alloy::primitives::Address::random()),
            to: Some(H160::from(alloy::primitives::Address::random())),
            transaction_type: Default::default(),
            cumulative_gas_used: rand::random::<u64>().into(),
            gas_price: Default::default(),
            max_fee_per_gas: Default::default(),
            max_priority_fee_per_gas: Default::default(),
            timestamp: 0,
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
    fn test_transaction_type_from_exe_result_to_transaction_receipt() {
        let mut exe_result = StorableExecutionResult {
            exe_result: ExeResult::Halt {
                gas_used: rand::random::<u64>().into(),
                error: crate::HaltError::PriorityFeeGreaterThanMaxFee,
            },
            transaction_hash: H256::from(alloy::primitives::B256::random()),
            transaction_index: rand::random::<u64>().into(),
            block_hash: H256::from(alloy::primitives::B256::random()),
            block_number: rand::random::<u64>().into(),
            from: H160::from(alloy::primitives::Address::random()),
            to: Some(H160::from(alloy::primitives::Address::random())),
            transaction_type: None,
            cumulative_gas_used: rand::random::<u64>().into(),
            gas_price: Default::default(),
            max_fee_per_gas: Default::default(),
            max_priority_fee_per_gas: Default::default(),
            timestamp: 0,
        };

        // Legacy TX type
        {
            let mut exe_result = exe_result.clone();
            exe_result.transaction_type = None;
            let receipt: TransactionReceipt = exe_result.into();
            assert_eq!(receipt.transaction_type, Some(U64::zero()));
        }

        // TX type 2
        {
            exe_result.transaction_type = Some(2u64.into());
            let receipt: TransactionReceipt = exe_result.into();
            assert_eq!(receipt.transaction_type, Some(2u64.into()));
        }
    }

    #[test]
    fn primitive_signature_roundtrip() {
        let signature = alloy::primitives::PrimitiveSignature::new(
            alloy::primitives::U256::from(random::<u64>()),
            alloy::primitives::U256::from(random::<u64>()),
            random(),
        );

        let roundtrip_signature = Signature::from(signature);
        assert_eq!(
            signature,
            alloy::primitives::PrimitiveSignature::from(roundtrip_signature)
        );
    }

    #[test]
    fn test_signature_malleability_check() {
        let s = U256::from_hex_str(SIGNATURE_S_UPPER_LIMIT_HEX_STR).unwrap();
        Signature::check_malleability(&s).unwrap();

        // If signature S field exceeds the limit, it should return an error.
        Signature::check_malleability(&(s + U256::from(1u64))).unwrap_err();
    }

    #[test]
    fn test_tx_rlp_encoding_should_preserve_signature_v() {
        let mut tx = Transaction {
            gas_price: Some(U256::from(1u64)),
            transaction_type: Some(0u64.into()),
            v: 27u64.into(),
            r: U256::from(1u64),
            s: U256::from(1u64),
            ..Default::default()
        };

        tx.hash = tx.slow_hash().unwrap().0;

        let rlp = tx.rlp_encoded_2718().unwrap();
        let tx_from_rlp = Transaction::from_rlp_2718(&mut rlp.as_ref()).unwrap();

        assert_eq!(tx, tx_from_rlp);
    }

    #[test]
    fn test_tx_rlp_encoding_should_preserve_signature_v_for_eip1559() {
        // This TX is from testnet block 5,169,760 (probably already reverted).
        // It contains a legacy transaction with a wrong signature V value due to unneeded normalization of the value.
        let tx = r#"
        {
            "blockHash": "0xaf2e9d3584c10c1a82500911027fe64edb892d36c650d68dcd063293cc08e638",
            "blockNumber": "0x4ee260",
            "chainId": "0x56b29",
            "from": "0xb0e5863d0ddf7e105e409fee0ecc0123a362e14b",
            "gas": "0xb71b00",
            "gasPrice": "0xe",
            "hash": "0x647bef21f7b58209d202e92d719ad5670aee3fb9a7bc70ddc5245fd8889e2e11",
            "input": "0x",
            "nonce": "0xf9ce30",
            "r": "0x1d811034f7f6940c4271b3a1b6c7d479ae23fd765e80e90c40c98901b0d6f48a",
            "s": "0x2d04ad2494a29743e6c8b85825ea799d71f75ebf2cfcc72efa0315fe901a1568",
            "to": "0x36c4054a98366caef1abb11469b9519e24b3cb78",
            "transactionIndex": "0x0",
            "type": "0x0",
            "v": "0x1",
            "value": "0xde0b6b3a7640000"
        }
        "#;

        let transaction: Transaction = serde_json::from_str(tx).unwrap();

        assert_eq!(transaction.hash, transaction.slow_hash().unwrap().0);

        // from/to rlp
        {
            let (hash, rlp) = transaction.slow_hash().unwrap();
            let tx_from_rlp = Transaction::from_rlp_2718(&mut rlp.as_ref()).unwrap();

            // rlp decoded TX should have the hash set
            assert_eq!(hash, tx_from_rlp.hash);

            let (re_hash, re_rlp) = tx_from_rlp.slow_hash().unwrap();
            assert_eq!(hash, re_hash);
            assert_eq!(rlp, re_rlp);

            assert_eq!(transaction.s, tx_from_rlp.s);
            assert_eq!(transaction.r, tx_from_rlp.r);

            // This is the expected V value, it corresponds to: (chain_id * 2) + 35 + signature_y_parity
            let expected_v = (355113u64 * 2) + 35 + 1;

            assert_eq!(expected_v, tx_from_rlp.v.as_u64());
        }
    }

    #[test]
    pub fn test_parity_to_from_bool() {
        assert_eq!(Parity::from_y_parity_is_odd(true), Parity::Odd);
        assert_eq!(Parity::from_y_parity_is_odd(false), Parity::Even);

        assert!(Parity::Odd.as_bool());
        assert!(!Parity::Even.as_bool());

        let parity = Parity::from_y_parity_is_odd(true);
        assert!(parity.as_bool());

        let parity = Parity::from_y_parity_is_odd(false);
        assert!(!parity.as_bool());
    }

    #[test]
    pub fn test_signature_v_for_legacy_eip155_transaction_with_y_parity_true() {
        // Arrange
        let signature = Signature::new_from_rsv(100u64.into(), 200u64.into(), 1u64).unwrap();
        assert_eq!(signature.r, 100u64.into());
        assert_eq!(signature.s, 200u64.into());
        assert_eq!(Parity::Odd, signature.y_parity);
        assert!(signature.y_parity.as_bool());

        let chain_id = random::<u32>() as u64;

        // Act
        let v = signature.v(TxChainInfo::LegacyTx {
            chain_id: Some(chain_id),
        });

        // Assert
        let expected_v = 35 + (chain_id * 2) + (signature.y_parity as u64);
        assert_eq!(v, expected_v);
    }

    #[test]
    pub fn test_signature_v_for_legacy_eip155_transaction_with_y_parity_false() {
        // Arrange
        let signature = Signature::new_from_rsv(300u64.into(), 500u64.into(), 0u64).unwrap();
        assert_eq!(signature.r, 300u64.into());
        assert_eq!(signature.s, 500u64.into());
        assert_eq!(Parity::Even, signature.y_parity);
        assert!(!signature.y_parity.as_bool());

        let chain_id = random::<u32>() as u64;

        // Act
        let v = signature.v(TxChainInfo::LegacyTx {
            chain_id: Some(chain_id),
        });

        // Assert
        let expected_v = 35 + (chain_id * 2) + (signature.y_parity as u64);
        assert_eq!(v, expected_v);
    }

    #[test]
    pub fn test_signature_v_for_legacy_not_eip155_transaction_with_y_parity_true() {
        // Arrange
        let signature = Signature::new_from_rsv(100u64.into(), 200u64.into(), 28u64).unwrap();
        assert_eq!(signature.r, 100u64.into());
        assert_eq!(signature.s, 200u64.into());
        assert_eq!(Parity::Odd, signature.y_parity);
        assert!(signature.y_parity.as_bool());

        // Act
        let v = signature.v(TxChainInfo::LegacyTx { chain_id: None });

        // Assert
        let expected_v = 28;
        assert_eq!(v, expected_v);
    }

    #[test]
    pub fn test_signature_v_for_legacy_not_eip155_transaction_with_y_parity_false() {
        // Arrange
        let signature = Signature::new_from_rsv(1000u64.into(), 2000u64.into(), 27u64).unwrap();
        assert_eq!(signature.r, 1000u64.into());
        assert_eq!(signature.s, 2000u64.into());
        assert_eq!(Parity::Even, signature.y_parity);
        assert!(!signature.y_parity.as_bool());

        // Act
        let v = signature.v(TxChainInfo::LegacyTx { chain_id: None });

        // Assert
        let expected_v = 27;
        assert_eq!(v, expected_v);
    }

    #[test]
    pub fn test_signature_v_for_non_legacy_transaction_with_y_parity_true() {
        // Arrange
        let signature = Signature::new_from_rsv(100u64.into(), 200u64.into(), 100u64).unwrap();
        assert_eq!(signature.r, 100u64.into());
        assert_eq!(signature.s, 200u64.into());
        assert_eq!(Parity::Odd, signature.y_parity);
        assert!(signature.y_parity.as_bool());

        // Act
        let v = signature.v(TxChainInfo::OtherTx);

        // Assert
        let expected_v = 1;
        assert_eq!(v, expected_v);
    }

    #[test]
    pub fn test_signature_v_for_non_legacy_transaction_with_y_parity_false() {
        // Arrange
        let signature = Signature::new_from_rsv(1000u64.into(), 2000u64.into(), 101u64).unwrap();
        assert_eq!(signature.r, 1000u64.into());
        assert_eq!(signature.s, 2000u64.into());
        assert_eq!(Parity::Even, signature.y_parity);
        assert!(!signature.y_parity.as_bool());

        // Act
        let v = signature.v(TxChainInfo::OtherTx);

        // Assert
        let expected_v = 0;
        assert_eq!(v, expected_v);
    }

    #[test]
    pub fn test_signature_creation_should_fail_if_not_valid_v() {
        assert!(Signature::new_from_rsv(100u64.into(), 200u64.into(), 2u64).is_err());
        assert!(Signature::new_from_rsv(100u64.into(), 200u64.into(), 29u64).is_err());
        assert!(Signature::new_from_rsv(100u64.into(), 200u64.into(), 32u64).is_err());
    }
}
