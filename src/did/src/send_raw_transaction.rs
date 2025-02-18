use alloy::primitives::FixedBytes;
use candid::CandidType;
use serde::{Deserialize, Serialize};

use crate::constant::{
    TRANSACTION_TYPE_EIP1559, TRANSACTION_TYPE_EIP2930, TRANSACTION_TYPE_LEGACY,
};
use crate::hash::Hash;
use crate::transaction::AccessList;
use crate::{Bytes, Transaction, H160, U256, U64};

/// A transaction is a single cryptographically signed instruction sent by an external account.
#[derive(Debug, Clone, PartialEq, Eq, CandidType, Serialize, Deserialize)]
pub enum UserTransaction {
    Legacy(TxLegacy),
    Eip2930(TxEip2930),
    Eip1559(TxEip1559),
}

impl From<UserTransaction> for Transaction {
    fn from(value: UserTransaction) -> Self {
        match value {
            UserTransaction::Legacy(tx) => tx.into(),
            UserTransaction::Eip2930(tx) => tx.into(),
            UserTransaction::Eip1559(tx) => tx.into(),
        }
    }
}

/// Legacy transaction format
#[derive(Debug, Clone, PartialEq, Eq, CandidType, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TxLegacy {
    /// Transaction hash
    pub hash: Hash<FixedBytes<32>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Added as EIP-155: Simple replay attack protection
    pub chain_id: Option<U256>,
    /// A scalar value equal to the number of transactions sent by the sender; formally Tn.
    pub nonce: U256,
    /// A scalar value equal to the number of
    /// Wei to be paid per unit of gas for all computation
    /// costs incurred as a result of the execution of this transaction; formally Tp.
    ///
    /// As ethereum circulation is around 120mil eth as of 2022 that is around
    /// 120000000000000000000000000 wei we are safe to use u128 as its max number is:
    /// 340282366920938463463374607431768211455
    pub gas_price: U256,
    /// A scalar value equal to the maximum
    /// amount of gas that should be used in executing
    /// this transaction. This is paid up-front, before any
    /// computation is done and may not be increased
    /// later; formally Tg.
    pub gas_limit: U256,
    /// The 160-bit address of the message call’s sender; formally Ts.
    #[serde(default)]
    pub from: H160,
    /// The 160-bit address of the message call’s recipient or, for a contract creation
    /// transaction, ∅, used here to denote the only member of B0 ; formally Tt.
    #[serde(default)]
    pub to: Option<H160>,
    /// A scalar value equal to the number of Wei to
    /// be transferred to the message call’s recipient or,
    /// in the case of contract creation, as an endowment
    /// to the newly created account; formally Tv.
    pub value: U256,
    /// Input has two uses depending if transaction is Create or Call (if `to` field is None or
    /// Some). pub init: An unlimited size byte array specifying the
    /// EVM-code for the account initialisation procedure CREATE,
    /// data: An unlimited size byte array specifying the
    /// input data of the message call, formally Td.
    pub input: Bytes,
    /// ECDSA recovery id
    pub v: U64,
    /// ECDSA signature r
    pub r: U256,
    /// ECDSA signature s
    pub s: U256,
}

impl From<TxLegacy> for Transaction {
    fn from(value: TxLegacy) -> Self {
        Transaction {
            transaction_type: Some(TRANSACTION_TYPE_LEGACY.into()),
            chain_id: value.chain_id,
            hash: value.hash,
            nonce: value.nonce,
            from: value.from,
            to: value.to,
            value: value.value,
            input: value.input,
            v: value.v,
            r: value.r,
            s: value.s,
            gas_price: Some(value.gas_price),
            gas: value.gas_limit,
            block_hash: None,
            block_number: None,
            transaction_index: None,
            access_list: None,
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
        }
    }
}

/// EIP-2930 transaction format
#[derive(Debug, Clone, PartialEq, Eq, CandidType, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TxEip2930 {
    /// Transaction hash
    pub hash: Hash<FixedBytes<32>>,
    /// Added as EIP-pub 155: Simple replay attack protection
    pub chain_id: U256,
    /// A scalar value equal to the number of transactions sent by the sender; formally Tn.
    pub nonce: U256,
    /// A scalar value equal to the number of
    /// Wei to be paid per unit of gas for all computation
    /// costs incurred as a result of the execution of this transaction; formally Tp.
    ///
    /// As ethereum circulation is around 120mil eth as of 2022 that is around
    /// 120000000000000000000000000 wei we are safe to use u128 as its max number is:
    /// 340282366920938463463374607431768211455
    pub gas_price: U256,
    /// A scalar value equal to the maximum
    /// amount of gas that should be used in executing
    /// this transaction. This is paid up-front, before any
    /// computation is done and may not be increased
    /// later; formally Tg.
    pub gas_limit: U256,
    /// The 160-bit address of the message call’s sender; formally Ts.
    #[serde(default)]
    pub from: H160,
    /// The 160-bit address of the message call’s recipient or, for a contract creation
    /// transaction, ∅, used here to denote the only member of B0 ; formally Tt.
    #[serde(default)]
    pub to: Option<H160>,
    /// A scalar value equal to the number of Wei to
    /// be transferred to the message call’s recipient or,
    /// in the case of contract creation, as an endowment
    /// to the newly created account; formally Tv.
    pub value: U256,
    /// The accessList specifies a list of addresses and storage keys;
    /// these addresses and storage keys are added into the `accessed_addresses`
    /// and `accessed_storage_keys` global sets (introduced in EIP-2929).
    /// A gas cost is charged, though at a discount relative to the cost of
    /// accessing outside the list.
    pub access_list: AccessList,
    /// Input has two uses depending if transaction is Create or Call (if `to` field is None or
    /// Some). pub init: An unlimited size byte array specifying the
    /// EVM-code for the account initialisation procedure CREATE,
    /// data: An unlimited size byte array specifying the
    /// input data of the message call, formally Td.
    pub input: Bytes,
    /// ECDSA recovery id
    pub v: U64,
    /// ECDSA signature r
    pub r: U256,
    /// ECDSA signature s
    pub s: U256,
}

impl From<TxEip2930> for Transaction {
    fn from(value: TxEip2930) -> Self {
        Transaction {
            transaction_type: Some(TRANSACTION_TYPE_EIP2930.into()),
            chain_id: Some(value.chain_id),
            hash: value.hash,
            nonce: value.nonce,
            from: value.from,
            to: value.to,
            value: value.value,
            input: value.input,
            v: value.v,
            r: value.r,
            s: value.s,
            gas_price: Some(value.gas_price),
            gas: value.gas_limit,
            access_list: Some(value.access_list),
            block_hash: None,
            block_number: None,
            transaction_index: None,
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
        }
    }
}

/// EIP-1559 transaction format
#[derive(Debug, Clone, PartialEq, Eq, CandidType, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TxEip1559 {
    /// Transaction hash
    pub hash: Hash<FixedBytes<32>>,
    /// EIP-155: Simple replay attack protection
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chain_id: Option<U256>,
    /// A scalar value equal to the number of transactions sent by the sender; formally Tn.
    pub nonce: U256,
    /// A scalar value equal to the maximum
    /// amount of gas that should be used in executing
    /// this transaction. This is paid up-front, before any
    /// computation is done and may not be increased
    /// later; formally Tg.
    pub gas_limit: U256,
    /// A scalar value equal to the maximum
    /// amount of gas that should be used in executing
    /// this transaction. This is paid up-front, before any
    /// computation is done and may not be increased
    /// later; formally Tg.
    ///
    /// As ethereum circulation is around 120mil eth as of 2022 that is around
    /// 120000000000000000000000000 wei we are safe to use u128 as its max number is:
    /// 340282366920938463463374607431768211455
    ///
    /// This is also known as `GasFeeCap`
    pub max_fee_per_gas: U256,
    /// Max Priority fee that transaction is paying
    ///
    /// As ethereum circulation is around 120mil eth as of 2022 that is around
    /// 120000000000000000000000000 wei we are safe to use u128 as its max number is:
    /// 340282366920938463463374607431768211455
    ///
    /// This is also known as `GasTipCap`
    pub max_priority_fee_per_gas: U256,
    /// The 160-bit address of the message call’s sender; formally Ts.
    #[serde(default)]
    pub from: H160,
    /// The 160-bit address of the message call’s recipient or, for a contract creation
    /// transaction, ∅, used here to denote the only member of B0 ; formally Tt.
    #[serde(default)]
    pub to: Option<H160>,
    /// A scalar value equal to the number of Wei to
    /// be transferred to the message call’s recipient or,
    /// in the case of contract creation, as an endowment
    /// to the newly created account; formally Tv.
    pub value: U256,
    /// The accessList specifies a list of addresses and storage keys;
    /// these addresses and storage keys are added into the `accessed_addresses`
    /// and `accessed_storage_keys` global sets (introduced in EIP-2929).
    /// A gas cost is charged, though at a discount relative to the cost of
    /// accessing outside the list.
    pub access_list: AccessList,
    /// Input has two uses depending if transaction is Create or Call (if `to` field is None or
    /// Some). pub init: An unlimited size byte array specifying the
    /// EVM-code for the account initialisation procedure CREATE,
    /// data: An unlimited size byte array specifying the
    /// input data of the message call, formally Td.
    pub input: Bytes,
    /// ECDSA recovery id
    pub v: U64,
    /// ECDSA signature r
    pub r: U256,
    /// ECDSA signature s
    pub s: U256,
}

impl From<TxEip1559> for Transaction {
    fn from(value: TxEip1559) -> Self {
        Transaction {
            transaction_type: Some(TRANSACTION_TYPE_EIP1559.into()),
            chain_id: value.chain_id,
            hash: value.hash,
            nonce: value.nonce,
            from: value.from,
            to: value.to,
            value: value.value,
            input: value.input,
            v: value.v,
            r: value.r,
            s: value.s,
            gas: value.gas_limit,
            access_list: Some(value.access_list),
            max_fee_per_gas: Some(value.max_fee_per_gas),
            max_priority_fee_per_gas: Some(value.max_priority_fee_per_gas),
            gas_price: None,
            block_hash: None,
            block_number: None,
            transaction_index: None,
        }
    }
}
