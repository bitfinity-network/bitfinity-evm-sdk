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

impl TryFrom<Transaction> for UserTransaction {
    type Error = &'static str;

    fn try_from(value: Transaction) -> Result<Self, Self::Error> {
        match value.transaction_type.map(|val| val.as_u64()) {
            Some(TRANSACTION_TYPE_EIP1559) => Ok(Self::Eip1559(TxEip1559::try_from(value)?)),
            Some(TRANSACTION_TYPE_EIP2930) => Ok(Self::Eip2930(TxEip2930::try_from(value)?)),
            None | Some(TRANSACTION_TYPE_LEGACY) => Ok(Self::Legacy(TxLegacy::try_from(value)?)),
            _ => Err("Unknown transaction type"),
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

impl TryFrom<Transaction> for TxLegacy {
    type Error = &'static str;

    fn try_from(value: Transaction) -> Result<Self, Self::Error> {
        Ok(Self {
            chain_id: value.chain_id,
            hash: value.hash,
            nonce: value.nonce,
            gas_price: value.gas_price.ok_or("Missing gas price")?,
            gas_limit: value.gas,
            from: value.from,
            to: value.to,
            value: value.value,
            input: value.input,
            v: value.v,
            r: value.r,
            s: value.s,
        })
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

impl TryFrom<Transaction> for TxEip2930 {
    type Error = &'static str;

    fn try_from(value: Transaction) -> Result<Self, Self::Error> {
        Ok(Self {
            hash: value.hash,
            chain_id: value.chain_id.ok_or("Missing chain id")?,
            nonce: value.nonce,
            gas_price: value.gas_price.ok_or("Missing gas price")?,
            gas_limit: value.gas,
            from: value.from,
            to: value.to,
            value: value.value,
            access_list: value.access_list.ok_or("Missing access list")?,
            input: value.input,
            v: value.v,
            r: value.r,
            s: value.s,
        })
    }
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

impl TryFrom<Transaction> for TxEip1559 {
    type Error = &'static str;

    fn try_from(value: Transaction) -> Result<Self, Self::Error> {
        Ok(Self {
            hash: value.hash,
            chain_id: value.chain_id,
            nonce: value.nonce,
            gas_limit: value.gas,
            max_fee_per_gas: value.max_fee_per_gas.ok_or("Missing max fee per gas")?,
            max_priority_fee_per_gas: value
                .max_priority_fee_per_gas
                .ok_or("Missing max priority fee per gas")?,
            from: value.from,
            to: value.to,
            value: value.value,
            access_list: value.access_list.ok_or("Missing access list")?,
            input: value.input,
            v: value.v,
            r: value.r,
            s: value.s,
        })
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::transaction::AccessListItem;

    #[test]
    fn test_should_convert_transaction_to_user_transaction_for_legacy() {
        let legacy = TxLegacy {
            hash: Hash::<FixedBytes<32>>::from_hex_str(
                "647bef21f7b58209d202e92d719ad5670aee3fb9a7bc70ddc5245fd8889e2e11",
            )
            .expect("Failed to parse hash"),
            chain_id: Some(U256::from(5u64)),
            nonce: U256::from(1u64),
            gas_price: U256::from(10u64),
            gas_limit: U256::from(27_000u64),
            from: H160::default(),
            to: Some(H160::default()),
            value: U256::from(10_000_000_000u64),
            input: Bytes::default(),
            v: U64::from(4722869645213696u64),
            r: U256::from(2036234056283528097u64),
            s: U256::from(3946284991422819502u64),
        };

        let transaction = Transaction::from(legacy.clone());

        // convert back
        let legacy_check = TxLegacy::try_from(transaction.clone()).expect("Failed to convert");

        assert_eq!(legacy, legacy_check);
    }

    #[test]
    fn test_should_convert_transaction_to_user_transaction_for_eip2930() {
        let eip2930 = TxEip2930 {
            hash: Hash::<FixedBytes<32>>::from_hex_str(
                "647bef21f7b58209d202e92d719ad5670aee3fb9a7bc70ddc5245fd8889e2e11",
            )
            .expect("Failed to parse hash"),
            chain_id: U256::from(5u64),
            nonce: U256::from(1u64),
            gas_price: U256::from(10u64),
            gas_limit: U256::from(27_000u64),
            from: H160::default(),
            to: Some(H160::default()),
            value: U256::from(10_000_000_000u64),
            access_list: AccessList(vec![AccessListItem {
                address: alloy::primitives::Address::random().into(),
                storage_keys: vec![alloy::primitives::B256::random().into()],
            }]),
            input: Bytes::default(),
            v: U64::from(4722869645213696u64),
            r: U256::from(2036234056283528097u64),
            s: U256::from(3946284991422819502u64),
        };

        let transaction = Transaction::from(eip2930.clone());

        // convert back
        let eip2930_check = TxEip2930::try_from(transaction.clone()).expect("Failed to convert");

        assert_eq!(eip2930, eip2930_check);
    }

    #[test]
    fn test_should_convert_transaction_to_user_transaction_for_eip1559() {
        let eip1559 = TxEip1559 {
            hash: Hash::<FixedBytes<32>>::from_hex_str(
                "647bef21f7b58209d202e92d719ad5670aee3fb9a7bc70ddc5245fd8889e2e11",
            )
            .expect("Failed to parse hash"),
            chain_id: Some(U256::from(5u64)),
            nonce: U256::from(1u64),
            gas_limit: U256::from(27_000u64),
            max_fee_per_gas: U256::from(10u64),
            max_priority_fee_per_gas: U256::from(5u64),
            from: H160::default(),
            to: Some(H160::default()),
            value: U256::from(10_000_000_000u64),
            access_list: AccessList(vec![AccessListItem {
                address: alloy::primitives::Address::random().into(),
                storage_keys: vec![alloy::primitives::B256::random().into()],
            }]),
            input: Bytes::default(),
            v: U64::from(4722869645213696u64),
            r: U256::from(2036234056283528097u64),
            s: U256::from(3946284991422819502u64),
        };

        let transaction = Transaction::from(eip1559.clone());

        // convert back
        let eip1559_check = TxEip1559::try_from(transaction.clone()).expect("Failed to convert");

        assert_eq!(eip1559, eip1559_check);
    }
}
