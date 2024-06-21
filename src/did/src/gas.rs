use candid::CandidType;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::transaction::AccessList;
use crate::{Bytes, H160, U256};

#[derive(Debug, Clone, Default, Eq, PartialEq, Serialize, CandidType, Deserialize)]
/// The `estimate_gas` method parameters
pub struct EstimateGasRequest {
    pub from: Option<H160>,
    pub to: Option<H160>,
    #[serde(rename = "gasPrice", default, skip_serializing_if = "Option::is_none")]
    pub gas_price: Option<U256>,
    /// EIP-1559 Max base fee the caller is willing to pay
    #[serde(
        rename = "maxFeePerGas",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub max_fee_per_gas: Option<U256>,
    /// EIP-1559 Priority fee the caller is paying to the block author
    #[serde(
        rename = "maxPriorityFeePerGas",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub max_priority_fee_per_gas: Option<U256>,
    pub gas: Option<U256>,
    pub value: Option<U256>,
    #[serde(default, flatten)]
    pub data: TransactionInput,
    pub nonce: Option<U256>,
    #[serde(rename = "chainId", default, skip_serializing_if = "Option::is_none")]
    pub chain_id: Option<U256>,
    #[serde(
        rename = "accessList",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub access_list: Option<AccessList>,
}

/// Helper type that supports both `data` and `input` fields that map to transaction input data.
///
/// This is done for compatibility reasons where older implementations used `data` instead of the
/// newer, recommended `input` field.
///
/// If both fields are set, it is expected that they contain the same value, otherwise an error is
/// returned.
#[derive(Clone, Debug, Default, PartialEq, Eq, CandidType, Serialize, Deserialize)]
pub struct TransactionInput {
    /// Transaction data
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<Bytes>,
    /// Transaction data
    ///
    /// This is the same as `input` but is used for backwards compatibility: <https://github.com/ethereum/go-ethereum/issues/15628>
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Bytes>,
}

impl TransactionInput {
    /// Consumes the type and returns the optional input data.
    #[inline]
    pub fn into_input(self) -> Option<Bytes> {
        self.input.or(self.data)
    }

    /// Returns the optional input data.
    #[inline]
    pub fn input(&self) -> Option<&Bytes> {
        self.input.as_ref().or(self.data.as_ref())
    }

    /// Returns the optional input data.
    ///
    /// Returns an error if both `data` and `input` fields are set and not equal.
    #[inline]
    pub fn unique_input(&self) -> Result<Option<&Bytes>> {
        self.check_unique_input().map(|()| self.input())
    }

    fn check_unique_input(&self) -> Result<()> {
        if let (Some(input), Some(data)) = (&self.input, &self.data) {
            if input != data {
                return Err(crate::error::EvmError::TransactionInputError(
                    "input and data fields differ".to_string(),
                )
                .into());
            }
        }
        Ok(())
    }
}

impl From<Vec<u8>> for TransactionInput {
    fn from(input: Vec<u8>) -> Self {
        Self {
            input: Some(input.into()),
            data: None,
        }
    }
}

impl From<Bytes> for TransactionInput {
    fn from(input: Bytes) -> Self {
        Self {
            input: Some(input),
            data: None,
        }
    }
}

impl From<Option<Bytes>> for TransactionInput {
    fn from(input: Option<Bytes>) -> Self {
        Self { input, data: None }
    }
}
