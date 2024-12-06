use alloy::consensus::{SignableTransaction, Transaction, TxEip1559, TxEip2930, TxLegacy};
use alloy::eips::eip2718::Eip2718Error;
use alloy::primitives::Parity;

use crate::constant::{
    TRANSACTION_TYPE_EIP1559, TRANSACTION_TYPE_EIP2930, TRANSACTION_TYPE_LEGACY,
};
use crate::error::EvmError;
use crate::transaction::{AccessList, AccessListItem, Signature};
use crate::{Bytes, H160, H256, H64, U256, U64};

impl From<alloy::primitives::Bytes> for Bytes {
    fn from(value: alloy::primitives::Bytes) -> Self {
        Bytes(value.0)
    }
}

impl From<Bytes> for alloy::primitives::Bytes {
    fn from(value: Bytes) -> Self {
        alloy::primitives::Bytes(value.0)
    }
}

impl From<alloy::primitives::Address> for H160 {
    fn from(value: alloy::primitives::Address) -> Self {
        H160::from_slice(value.as_slice())
    }
}

impl From<H160> for alloy::primitives::Address {
    fn from(value: H160) -> Self {
        alloy::primitives::Address::from_slice(value.0.as_bytes())
    }
}

impl From<alloy::primitives::B64> for H64 {
    fn from(value: alloy::primitives::B64) -> Self {
        H64::from_slice(value.as_slice())
    }
}

impl From<H64> for alloy::primitives::B64 {
    fn from(value: H64) -> Self {
        alloy::primitives::B64::from_slice(value.0.as_bytes())
    }
}

impl From<alloy::primitives::B256> for H256 {
    fn from(value: alloy::primitives::B256) -> Self {
        H256::from_slice(value.as_slice())
    }
}

impl From<H256> for alloy::primitives::B256 {
    fn from(value: H256) -> Self {
        alloy::primitives::B256::from_slice(value.0.as_bytes())
    }
}

impl From<alloy::primitives::U256> for U256 {
    fn from(value: alloy::primitives::U256) -> Self {
        U256::from_little_endian(value.as_le_slice())
    }
}

impl From<U256> for alloy::primitives::U256 {
    fn from(value: U256) -> Self {
        let mut bytes = [0u8; U256::BYTE_SIZE];
        value.0.to_little_endian(&mut bytes);
        alloy::primitives::U256::from_le_bytes(bytes)
    }
}

impl From<alloy::primitives::U64> for U64 {
    fn from(value: alloy::primitives::U64) -> Self {
        U64::from_little_endian(value.as_le_slice())
    }
}

impl From<U64> for alloy::primitives::U64 {
    fn from(value: U64) -> Self {
        let mut bytes = [0u8; U64::BYTE_SIZE];
        value.0.to_little_endian(&mut bytes);
        alloy::primitives::U64::from_le_bytes(bytes)
    }
}

impl From<alloy::consensus::TxEnvelope> for crate::Transaction {
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

impl From<crate::Transaction> for alloy::consensus::TxEnvelope {
    fn from(value: crate::Transaction) -> Self {
        let tx: alloy::rpc::types::Transaction = value.into();
        tx.inner
    }
}

impl From<alloy::rpc::types::Transaction> for crate::Transaction {
    fn from(tx: alloy::rpc::types::Transaction) -> Self {
        let signature: Signature = (*tx.inner.signature()).into();

        match tx.inner {
            alloy::consensus::TxEnvelope::Legacy(signed) => {
                let inner_tx = signed.tx();
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
                    v: signature.v,
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
                    v: signature.v,
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
                    v: signature.v,
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

impl From<crate::Transaction> for alloy::rpc::types::Transaction {
    fn from(tx: crate::Transaction) -> Self {
        let signature = Signature {
            v: tx.v,
            r: tx.r,
            s: tx.s,
        };
        let signature = alloy::primitives::PrimitiveSignature::try_from(signature).unwrap();

        let tx_type = tx.transaction_type.unwrap_or_default().0.as_u64();
        match tx_type {
            TRANSACTION_TYPE_LEGACY => alloy::rpc::types::Transaction {
                inner: TxLegacy {
                    nonce: tx.nonce.0.as_u64(),
                    gas_price: tx.gas_price.map(|v| v.0.as_u128()).unwrap_or_default(),
                    gas_limit: tx.gas.0.as_u64(),
                    to: tx.to.map(|v| alloy::primitives::Address::from(v)).into(),
                    value: tx.value.into(),
                    input: tx.input.into(),
                    chain_id: tx.chain_id.map(|v| v.0.as_u64()),
                }
                .into_signed(signature)
                .into(),
                block_hash: tx.block_hash.map(Into::into),
                block_number: tx.block_number.map(Into::into),
                transaction_index: tx.transaction_index.map(Into::into),
                effective_gas_price: None,
                from: tx.from.into(),
            },
            TRANSACTION_TYPE_EIP2930 => alloy::rpc::types::Transaction {
                inner: TxEip2930 {
                    nonce: tx.nonce.0.as_u64(),
                    gas_price: tx.gas_price.map(|v| v.0.as_u128()).unwrap_or_default(),
                    gas_limit: tx.gas.0.as_u64(),
                    to: tx.to.map(|v| alloy::primitives::Address::from(v)).into(),
                    value: tx.value.into(),
                    input: tx.input.into(),
                    chain_id: tx.chain_id.map(|v| v.0.as_u64()).unwrap_or_default(),
                    access_list: tx.access_list.map(Into::into).unwrap_or_default(),
                }
                .into_signed(signature)
                .into(),
                block_hash: tx.block_hash.map(Into::into),
                block_number: tx.block_number.map(Into::into),
                transaction_index: tx.transaction_index.map(Into::into),
                effective_gas_price: None,
                from: tx.from.into(),
            },
            TRANSACTION_TYPE_EIP1559 => alloy::rpc::types::Transaction {
                inner: TxEip1559 {
                    nonce: tx.nonce.0.as_u64(),
                    gas_limit: tx.gas.0.as_u64(),
                    to: tx.to.map(|v| alloy::primitives::Address::from(v)).into(),
                    value: tx.value.into(),
                    input: tx.input.into(),
                    chain_id: tx.chain_id.map(|v| v.0.as_u64()).unwrap_or_default(),
                    max_fee_per_gas: tx
                        .max_fee_per_gas
                        .map(|v| v.0.as_u128())
                        .unwrap_or_default(),
                    max_priority_fee_per_gas: tx
                        .max_priority_fee_per_gas
                        .map(|v| v.0.as_u128())
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
            },
            _ => {
                panic!("Unsupported transaction type: {}", tx_type);
            }
        }
    }
}

impl TryFrom<Signature> for alloy::primitives::Signature {
    type Error = EvmError;

    fn try_from(value: Signature) -> Result<Self, Self::Error> {
        Self::try_from(&value)
    }
}

impl TryFrom<&Signature> for alloy::primitives::Signature {
    type Error = EvmError;

    fn try_from(value: &Signature) -> Result<Self, Self::Error> {
        Parity::try_from(value.v.0.as_u64())
            .map_err(|e| EvmError::InvalidSignatureParity(e.to_string()))
            .map(|parity| {
                alloy::primitives::Signature::new(
                    value.r.clone().into(),
                    value.s.clone().into(),
                    parity,
                )
            })
    }
}

impl From<alloy::primitives::Signature> for Signature {
    fn from(value: alloy::primitives::Signature) -> Self {
        Self {
            v: U64::from(value.v().to_u64()),
            r: value.r().into(),
            s: value.s().into(),
        }
    }
}

impl TryFrom<Signature> for alloy::primitives::PrimitiveSignature {
    type Error = EvmError;

    fn try_from(value: Signature) -> Result<Self, Self::Error> {
        Self::try_from(&value)
    }
}

impl TryFrom<&Signature> for alloy::primitives::PrimitiveSignature {
    type Error = EvmError;

    fn try_from(value: &Signature) -> Result<Self, Self::Error> {
        let parity = Parity::try_from(value.v.0.as_u64())
            .map_err(|e| EvmError::InvalidSignatureParity(e.to_string()))?;
        Ok(alloy::primitives::PrimitiveSignature::new(
            value.r.clone().into(),
            value.s.clone().into(),
            parity.y_parity(),
        ))
    }
}

impl From<alloy::primitives::PrimitiveSignature> for Signature {
    fn from(value: alloy::primitives::PrimitiveSignature) -> Self {
        Self {
            v: U64::from(value.v() as u64),
            r: value.r().into(),
            s: value.s().into(),
        }
    }
}

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

impl From<Eip2718Error> for EvmError {
    fn from(eip2718_error: Eip2718Error) -> Self {
        Self::RlpError(format!("EIP-2718 rlp error: {eip2718_error}"))
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::{U256, U64};

    #[test]
    fn test_alloy_bytes_roundtrip() {
        let value = Bytes(bytes::Bytes::from(vec![
            rand::random::<u8>(),
            rand::random::<u8>(),
            rand::random::<u8>(),
        ]));

        let alloy_bytes = alloy::primitives::Bytes::from(value.clone());
        let decoded_value = Bytes::from(alloy_bytes);

        assert_eq!(value, decoded_value);
    }

    #[test]
    fn test_alloy_address_roundtrip() {
        let value: H160 = ethereum_types::H160::random().into();

        let alloy_address = alloy::primitives::Address::from(value.clone());
        let decoded_value = H160::from(alloy_address);

        assert_eq!(value, decoded_value);
    }

    #[test]
    fn test_alloy_h256_roundtrip() {
        let value: H256 = ethereum_types::H256::random().into();

        let alloy_h256 = alloy::primitives::B256::from(value.clone());
        let decoded_value = H256::from(alloy_h256);

        assert_eq!(value, decoded_value);
    }

    #[test]
    fn test_alloy_h64_roundtrip() {
        let value: H64 = ethereum_types::H64::random().into();

        let alloy_h64 = alloy::primitives::B64::from(value.clone());
        let decoded_value = H64::from(alloy_h64);

        assert_eq!(value, decoded_value);
    }

    #[test]
    fn test_alloy_u256_roundtrip() {
        let value: U256 = ethereum_types::U256::from(rand::random::<u128>()).into();

        let alloy_u256: alloy::primitives::U256 = value.clone().into();
        let decoded_value: U256 = alloy_u256.into();

        assert_eq!(value, decoded_value);
    }

    #[test]
    fn test_alloy_u64_roundtrip() {
        let value: U64 = ethereum_types::U64::from(rand::random::<u64>()).into();

        let alloy_u64: alloy::primitives::U64 = value.into();
        let decoded_value: U64 = alloy_u64.into();

        assert_eq!(value, decoded_value);
    }
}
