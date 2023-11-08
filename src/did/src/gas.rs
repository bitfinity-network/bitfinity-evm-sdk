use candid::CandidType;
use serde::{Deserialize, Serialize};

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
    pub data: Option<Bytes>,
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
