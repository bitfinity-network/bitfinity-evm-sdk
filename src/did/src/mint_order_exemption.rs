use candid::{CandidType, Principal};
use ethers_core::abi::ethabi::Bytes;
use ethers_core::abi::{Error, Function, Param, ParamType, StateMutability, Token};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use crate::H256;

/// Function signature, used if transaction should notify
/// some principal with some transaction receipt.
#[allow(deprecated)] // need to initialize `constant` field
pub static MINT_ORDER_EXEMPTION: Lazy<Function> = Lazy::new(|| Function {
    name: "evm_canister_mint_order_exemption".into(),
    inputs: vec![
        Param {
            name: "tx_hash".into(),
            kind: ParamType::FixedBytes(32),
            internal_type: None,
        },
        Param {
            name: "principal".into(),
            kind: ParamType::FixedBytes(32),
            internal_type: None,
        },
        Param {
            name: "user_data".into(),
            kind: ParamType::Bytes,
            internal_type: None,
        },
    ],
    outputs: vec![],
    constant: None,
    state_mutability: StateMutability::NonPayable,
});

/// Structured input for notification transaction.
#[derive(Debug, Clone, Serialize, Deserialize, CandidType, PartialEq, Eq)]
pub struct MintOrderExemptionInput {
    pub about_tx: Option<H256>,
    pub receiver_canister: Principal,
    pub user_data: Vec<u8>,
}

impl MintOrderExemptionInput {
    /// Minimal input length for mint order exemption transaction.
    /// - [0..4] - function signature hash,
    /// - [4..36] - transaction about which we should notify,
    /// - [36..] - encoded principal to which mint order exemption will be sent.
    pub const MIN_INPUT_LEN: usize = 4 + 32 + MintOrderExemptionUserData::MIN_INPUT_LEN;

    /// Encode input for mint order exemption transaction.
    pub fn encode(self) -> Result<Bytes, Error> {
        let mut principal_vec = vec![self.receiver_canister.as_slice().len() as u8];
        principal_vec.extend_from_slice(self.receiver_canister.as_slice());
        principal_vec.resize(32, 0);

        let about_tx_data = self.about_tx.unwrap_or_default().0 .0.to_vec();
        MINT_ORDER_EXEMPTION.encode_input(&[
            Token::FixedBytes(about_tx_data),
            Token::FixedBytes(principal_vec),
            Token::Bytes(self.user_data),
        ])
    }

    /// Decode mint order exemption transaction data from raw trancaction input.
    pub fn decode(tx_input: &[u8]) -> Option<Self> {
        if tx_input.len() < Self::MIN_INPUT_LEN {
            return None;
        }

        let call_signature = &tx_input[..4];
        let call_input = &tx_input[4..];

        if call_signature != MINT_ORDER_EXEMPTION.short_signature() {
            return None;
        }

        let input = MINT_ORDER_EXEMPTION.decode_input(call_input).ok()?;
        let tx_hash = input.get(0)?.clone().into_fixed_bytes()?;
        let principal_data = input.get(1)?.clone().into_fixed_bytes()?;
        let user_data = input.get(2)?.clone().into_bytes()?;

        let principal_len = principal_data[0] as usize;
        if principal_data.len() < principal_len + 1 {
            return None;
        }
        let receiver_canister =
            Principal::try_from_slice(&principal_data[1..(principal_len + 1)]).ok()?;
        let about_tx = match tx_hash.iter().all(|v| *v == 0) {
            true => None,
            false => Some(H256::from_slice(&tx_hash)),
        };

        Some(Self {
            about_tx,
            receiver_canister,
            user_data,
        })
    }
}

/// Structured input for notification transaction.
#[derive(Debug, Clone, Serialize, Deserialize, CandidType, PartialEq, Eq)]
pub struct MintOrderExemptionUserData {
    pub weeks: u32,
    pub user: Principal,
}

impl MintOrderExemptionUserData {
    pub const MIN_INPUT_LEN: usize = 5;

    /// Decode mint order exemption transaction data from raw trancaction input.
    pub fn decode(tx_input: &[u8]) -> Option<Self> {
        if tx_input.len() < Self::MIN_INPUT_LEN {
            return None;
        }
        let weeks = tx_input[..4].try_into().ok().map(u32::from_le_bytes)?;
        let user = Principal::from_slice(&tx_input[4..]);

        Some(Self { user, weeks })
    }
}

#[cfg(test)]
mod tests {

    use candid::Principal;

    use super::*;
    use crate::mint_order_exemption::MintOrderExemptionUserData;
    use crate::H256;

    #[test]
    fn mint_order_exemption_encoding() {
        let user_principal = Principal::anonymous();

        let mut user_data: Vec<u8> = Vec::with_capacity(MintOrderExemptionInput::MIN_INPUT_LEN);
        user_data.extend_from_slice(&4_u32.to_le_bytes());
        user_data.extend_from_slice(user_principal.as_slice());

        let data = MintOrderExemptionInput {
            about_tx: Some(H256::from([1; 32])),
            receiver_canister: Principal::management_canister(),
            user_data,
        };

        let encoded = data.clone().encode().unwrap();
        let decoded = MintOrderExemptionInput::decode(&encoded).unwrap();

        let user_data = MintOrderExemptionUserData::decode(&decoded.user_data).unwrap();
        assert_eq!(user_data.user, user_principal);
        assert_eq!(user_data.weeks, 4);

        assert_eq!(decoded, data)
    }

    #[test]
    fn invalid_principal() {
        // data length is too big
        let principal_data = vec![42; 32];
        let encoded = MINT_ORDER_EXEMPTION
            .encode_input(&[
                Token::FixedBytes(H256::from([1; 32]).0 .0.into()),
                Token::FixedBytes(principal_data),
                Token::Bytes(vec![]),
            ])
            .unwrap();
        let decoded = MintOrderExemptionUserData::decode(&encoded);

        assert_eq!(decoded, None);
    }
}
