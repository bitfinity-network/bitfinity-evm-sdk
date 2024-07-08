use candid::CandidType;
use serde::{Deserialize, Serialize};

use crate::transaction::AccessList;
use crate::{Bytes, H160, U256};

#[derive(Debug, Clone, Default, Eq, PartialEq, CandidType, Deserialize, Serialize)]
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
    #[serde(default, alias = "data", skip_serializing_if = "Option::is_none")]
    pub input: Option<Bytes>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{test_candid_roundtrip, test_json_roundtrip};

    #[test]
    fn test_serde_roundtrip_with_data_field() {
        let json = r#"{
            "from": "0x1234567890123456789012345678901234567890",
            "to": "0x0987654321098765432109876543210987654321",
            "gasPrice": "0x1234",
            "value": "0x5678",
            "data": "0xabcdef",
            "nonce": "0x9",
            "chainId": "0x1"
        }"#;

        let request: EstimateGasRequest = serde_json::from_str(json).unwrap();
        test_json_roundtrip(&request);
    }

    #[test]
    fn test_serde_roundtrip_with_input_field() {
        let json = r#"{
            "from": "0x1234567890123456789012345678901234567890",
            "to": "0x0987654321098765432109876543210987654321",
            "gasPrice": "0x1234",
            "value": "0x5678",
            "input": "0xabcdef",
            "nonce": "0x9",
            "chainId": "0x1"
        }"#;

        let request: EstimateGasRequest = serde_json::from_str(json).unwrap();
        test_json_roundtrip(&request);
    }

    #[test]
    fn test_candid_roundtrip_with_data_field() {
        let json = r#"{
            "from": "0x1234567890123456789012345678901234567890",
            "to": "0x0987654321098765432109876543210987654321",
            "gasPrice": "0x1234",
            "value": "0x5678",
            "data": "0xabcdef",
            "nonce": "0x9",
            "chainId": "0x1"
        }"#;

        let request: EstimateGasRequest = serde_json::from_str(json).unwrap();
        test_candid_roundtrip(&request);
    }

    #[test]
    fn test_candid_roundtrip_with_input_field() {
        let json = r#"{
            "from": "0x1234567890123456789012345678901234567890",
            "to": "0x0987654321098765432109876543210987654321",
            "gasPrice": "0x1234",
            "value": "0x5678",
            "input": "0xabcdef",
            "nonce": "0x9",
            "chainId": "0x1"
        }"#;

        let request: EstimateGasRequest = serde_json::from_str(json).unwrap();
        test_candid_roundtrip(&request);
    }
}
