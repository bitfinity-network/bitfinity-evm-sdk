mod signature;
mod tx_kind;

use candid::CandidType;
use serde::{Deserialize, Serialize};

pub use self::signature::Signature;
pub use self::tx_kind::TxKind;
use crate::constant::{
    TRANSACTION_TYPE_EIP1559, TRANSACTION_TYPE_EIP2930, TRANSACTION_TYPE_LEGACY,
};
use crate::transaction::AccessList;
use crate::{Bytes, Transaction, U256};

/// `send_raw_transaction` request payload
#[derive(Debug, Clone, PartialEq, Eq, CandidType, Serialize, Deserialize)]
pub struct SendRawTransactionRequest {
    /// The signature of the transaction
    pub signature: Signature,
    /// The transaction data
    pub transaction: TransactionData,
}

impl TryFrom<Transaction> for SendRawTransactionRequest {
    type Error = &'static str;

    fn try_from(value: Transaction) -> Result<Self, Self::Error> {
        let signature = Signature {
            r: value.r,
            s: value.s,
            v: value.v,
        };

        let data = match value
            .transaction_type
            .map(|x| x.as_u64())
            .unwrap_or(TRANSACTION_TYPE_LEGACY)
        {
            TRANSACTION_TYPE_LEGACY => TransactionData::Legacy(Legacy {
                chain_id: value.chain_id,
                nonce: value.nonce,
                gas_price: value.gas_price.ok_or("Gas price is missing")?,
                gas_limit: value.gas,
                to: value.to.into(),
                value: value.value,
                input: value.input,
            }),
            TRANSACTION_TYPE_EIP1559 => TransactionData::Eip1559(Eip1559 {
                chain_id: value.chain_id,
                nonce: value.nonce,
                gas_limit: value.gas,
                max_fee_per_gas: value.max_fee_per_gas.ok_or("Max fee per gas is missing")?,
                max_priority_fee_per_gas: value
                    .max_priority_fee_per_gas
                    .ok_or("Max priority fee per gas is missing")?,
                to: value.to.into(),
                value: value.value,
                access_list: value.access_list.ok_or("Access list is missing")?,
                input: value.input,
            }),
            TRANSACTION_TYPE_EIP2930 => TransactionData::Eip2930(Eip2930 {
                chain_id: value.chain_id.ok_or("Chain id is missing")?,
                nonce: value.nonce,
                gas_price: value.gas_price.ok_or("Gas price is missing")?,
                gas_limit: value.gas,
                to: value.to.into(),
                value: value.value,
                access_list: value.access_list.ok_or("Access list is missing")?,
                input: value.input,
            }),
            _ => return Err("Unknown transaction type"),
        };

        Ok(SendRawTransactionRequest {
            signature,
            transaction: data,
        })
    }
}

/// Transaction type and data
#[derive(Debug, Clone, PartialEq, Eq, CandidType, Serialize, Deserialize)]
pub enum TransactionData {
    Legacy(Legacy),
    Eip2930(Eip2930),
    Eip1559(Eip1559),
}

/// Legacy transaction format
#[derive(Debug, Clone, PartialEq, Eq, CandidType, Serialize, Deserialize, Default)]
pub struct Legacy {
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
    /// The 160-bit address of the message call’s recipient or, for a contract creation
    /// transaction, ∅, used here to denote the only member of B0 ; formally Tt.
    pub to: TxKind,
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
}

/// EIP-2930 transaction format
#[derive(Debug, Clone, PartialEq, Eq, CandidType, Serialize, Deserialize, Default)]
pub struct Eip2930 {
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
    /// The 160-bit address of the message call’s recipient or, for a contract creation
    /// transaction, ∅, used here to denote the only member of B0 ; formally Tt.
    pub to: TxKind,
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
}

/// EIP-1559 transaction format
#[derive(Debug, Clone, PartialEq, Eq, CandidType, Serialize, Deserialize, Default)]
pub struct Eip1559 {
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
    /// The 160-bit address of the message call’s recipient or, for a contract creation
    /// transaction, ∅, used here to denote the only member of B0 ; formally Tt.
    pub to: TxKind,
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
}

#[cfg(test)]
mod test {

    use candid::{Decode, Encode};

    use super::*;
    use crate::transaction::AccessListItem;
    use crate::{H160, U64};

    #[test]
    fn test_should_candid_encode_decode_legacy_transaction() {
        let legacy = SendRawTransactionRequest {
            transaction: TransactionData::Legacy(Legacy {
                chain_id: Some(U256::from(5u64)),
                nonce: U256::from(1u64),
                gas_price: U256::from(10u64),
                gas_limit: U256::from(27_000u64),
                to: TxKind::Call(H160::default()),
                value: U256::from(10_000_000_000u64),
                input: vec![0xca, 0xfe].into(),
            }),
            signature: Signature {
                v: U64::from(4722869645213696u64),
                r: U256::from(2036234056283528097u64),
                s: U256::from(3946284991422819502u64),
            },
        };

        let encoded = Encode!(&legacy).expect("Failed to encode");
        let decoded = Decode!(&encoded, SendRawTransactionRequest).expect("Failed to decode");

        assert_eq!(legacy, decoded);
    }

    #[test]
    fn test_should_candid_encode_decode_eip2930() {
        let eip2930 = SendRawTransactionRequest {
            transaction: TransactionData::Eip2930(Eip2930 {
                chain_id: U256::from(5u64),
                nonce: U256::from(1u64),
                gas_price: U256::from(10u64),
                gas_limit: U256::from(27_000u64),
                to: TxKind::Create,
                value: U256::from(10_000_000_000u64),
                access_list: AccessList(vec![AccessListItem {
                    address: alloy::primitives::Address::random().into(),
                    storage_keys: vec![alloy::primitives::B256::random().into()],
                }]),
                input: Bytes::default(),
            }),
            signature: Signature {
                v: U64::from(4722869645213696u64),
                r: U256::from(2036234056283528097u64),
                s: U256::from(3946284991422819502u64),
            },
        };

        let encoded = Encode!(&eip2930).expect("Failed to encode");
        let decoded = Decode!(&encoded, SendRawTransactionRequest).expect("Failed to decode");

        assert_eq!(eip2930, decoded);
    }

    #[test]
    fn test_should_candid_encode_decode_eip1559() {
        let eip1559 = SendRawTransactionRequest {
            transaction: TransactionData::Eip1559(Eip1559 {
                chain_id: Some(U256::from(5u64)),
                nonce: U256::from(1u64),
                gas_limit: U256::from(27_000u64),
                max_fee_per_gas: U256::from(10u64),
                max_priority_fee_per_gas: U256::from(5u64),
                to: TxKind::Call(H160::default()),
                value: U256::from(10_000_000_000u64),
                access_list: AccessList(vec![AccessListItem {
                    address: alloy::primitives::Address::random().into(),
                    storage_keys: vec![alloy::primitives::B256::random().into()],
                }]),
                input: vec![0xca, 0xfe].into(),
            }),
            signature: Signature {
                v: U64::from(4722869645213696u64),
                r: U256::from(2036234056283528097u64),
                s: U256::from(3946284991422819502u64),
            },
        };

        let encoded = Encode!(&eip1559).expect("Failed to encode");
        let decoded = Decode!(&encoded, SendRawTransactionRequest).expect("Failed to decode");

        assert_eq!(eip1559, decoded);
    }

    #[test]
    fn test_should_convert_transaction_to_user_transaction_for_legacy() {
        let legacy = SendRawTransactionRequest {
            transaction: TransactionData::Legacy(Legacy {
                chain_id: Some(U256::from(5u64)),
                nonce: U256::from(1u64),
                gas_price: U256::from(10u64),
                gas_limit: U256::from(27_000u64),
                to: TxKind::Call(H160::default()),
                value: U256::from(10_000_000_000u64),
                input: vec![0xca, 0xfe].into(),
            }),
            signature: Signature {
                v: U64::from(4722869645213696u64),
                r: U256::from(2036234056283528097u64),
                s: U256::from(3946284991422819502u64),
            },
        };

        let transaction = convert_to_tx(legacy.clone());

        // convert back
        let legacy_check =
            SendRawTransactionRequest::try_from(transaction.clone()).expect("Failed to convert");

        assert_eq!(legacy, legacy_check);
    }

    #[test]
    fn test_should_convert_transaction_to_user_transaction_for_eip2930() {
        let eip2930 = SendRawTransactionRequest {
            transaction: TransactionData::Eip2930(Eip2930 {
                chain_id: U256::from(5u64),
                nonce: U256::from(1u64),
                gas_price: U256::from(10u64),
                gas_limit: U256::from(27_000u64),
                to: TxKind::Call(H160::default()),
                value: U256::from(10_000_000_000u64),
                access_list: AccessList(vec![AccessListItem {
                    address: alloy::primitives::Address::random().into(),
                    storage_keys: vec![alloy::primitives::B256::random().into()],
                }]),
                input: Bytes::default(),
            }),
            signature: Signature {
                v: U64::from(4722869645213696u64),
                r: U256::from(2036234056283528097u64),
                s: U256::from(3946284991422819502u64),
            },
        };

        let transaction = convert_to_tx(eip2930.clone());

        // convert back
        let eip2930_check =
            SendRawTransactionRequest::try_from(transaction.clone()).expect("Failed to convert");

        assert_eq!(eip2930, eip2930_check);
    }

    #[test]
    fn test_should_convert_transaction_to_user_transaction_for_eip1559() {
        let eip1559 = SendRawTransactionRequest {
            transaction: TransactionData::Eip1559(Eip1559 {
                chain_id: Some(U256::from(5u64)),
                nonce: U256::from(1u64),
                gas_limit: U256::from(27_000u64),
                max_fee_per_gas: U256::from(10u64),
                max_priority_fee_per_gas: U256::from(5u64),
                to: TxKind::Call(H160::default()),
                value: U256::from(10_000_000_000u64),
                access_list: AccessList(vec![AccessListItem {
                    address: alloy::primitives::Address::random().into(),
                    storage_keys: vec![alloy::primitives::B256::random().into()],
                }]),
                input: vec![0xca, 0xfe].into(),
            }),
            signature: Signature {
                v: U64::from(4722869645213696u64),
                r: U256::from(2036234056283528097u64),
                s: U256::from(3946284991422819502u64),
            },
        };

        let transaction = convert_to_tx(eip1559.clone());

        // convert back
        let eip1559_check =
            SendRawTransactionRequest::try_from(transaction.clone()).expect("Failed to convert");

        assert_eq!(eip1559, eip1559_check);
    }

    /// Test function which converts `SendRawTransactionRequest` to `Transaction`.
    fn convert_to_tx(value: SendRawTransactionRequest) -> Transaction {
        let mut tx = Transaction {
            r: value.signature.r,
            s: value.signature.s,
            v: value.signature.v,
            ..Default::default()
        };

        match value.transaction {
            TransactionData::Eip1559(eip1159) => {
                tx.transaction_type = Some(TRANSACTION_TYPE_EIP1559.into());
                tx.nonce = eip1159.nonce;
                tx.access_list = Some(eip1159.access_list);
                tx.chain_id = eip1159.chain_id;
                tx.gas = eip1159.gas_limit;
                tx.max_fee_per_gas = Some(eip1159.max_fee_per_gas);
                tx.max_priority_fee_per_gas = Some(eip1159.max_priority_fee_per_gas);
                tx.to = eip1159.to.to().cloned();
                tx.value = eip1159.value;
                tx.input = eip1159.input;
            }
            TransactionData::Eip2930(eip2930) => {
                tx.transaction_type = Some(TRANSACTION_TYPE_EIP2930.into());
                tx.nonce = eip2930.nonce;
                tx.access_list = Some(eip2930.access_list);
                tx.chain_id = Some(eip2930.chain_id);
                tx.gas = eip2930.gas_limit;
                tx.gas_price = Some(eip2930.gas_price);
                tx.to = eip2930.to.to().cloned();
                tx.value = eip2930.value;
                tx.input = eip2930.input;
            }
            TransactionData::Legacy(legacy) => {
                tx.transaction_type = Some(TRANSACTION_TYPE_LEGACY.into());
                tx.nonce = legacy.nonce;
                tx.chain_id = legacy.chain_id;
                tx.gas = legacy.gas_limit;
                tx.gas_price = Some(legacy.gas_price);
                tx.to = legacy.to.to().cloned();
                tx.value = legacy.value;
                tx.input = legacy.input;
            }
        }

        tx
    }
}
