use candid::{CandidType, Principal};
use ethers_core::abi::ethabi::Bytes;
use ethers_core::abi::{Error, Function, Param, ParamType, StateMutability, Token};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use crate::H256;

/// Function signature, used if transaction should notify
/// some principal with some transaction receipt.
#[allow(deprecated)] // need to initialize `constant` field
pub static NOTIFICATION: Lazy<Function> = Lazy::new(|| Function {
    name: "evm_canister_notification_needed".into(),
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
pub struct NotificationInput {
    pub about_tx: Option<H256>,
    pub receiver_canister: Principal,
    pub user_data: Vec<u8>,
}

impl NotificationInput {
    /// Minimal input length for notification transaction.
    /// - [0..4] - function signature hash,
    /// - [4..36] - transaction about which we should notify,
    /// - [36..68] - encoded principal to which notification will be sent.
    pub const MIN_INPUT_LEN: usize = 68;

    /// Encode input for notification transaction.
    pub fn encode(self) -> Result<Bytes, Error> {
        let mut principal_vec = vec![self.receiver_canister.as_slice().len() as u8];
        principal_vec.extend_from_slice(self.receiver_canister.as_slice());
        principal_vec.resize(32, 0);

        let about_tx_data = self.about_tx.unwrap_or_default().0 .0.to_vec();
        NOTIFICATION.encode_input(&[
            Token::FixedBytes(about_tx_data),
            Token::FixedBytes(principal_vec),
            Token::Bytes(self.user_data),
        ])
    }

    /// Decode notification transaction data from raw trancaction input.
    pub fn decode(tx_input: &[u8]) -> Option<Self> {
        if tx_input.len() < Self::MIN_INPUT_LEN {
            return None;
        }

        let call_signature = &tx_input[..4];
        let call_input = &tx_input[4..];

        if call_signature != NOTIFICATION.short_signature() {
            return None;
        }

        let input = NOTIFICATION.decode_input(call_input).ok()?;
        let tx_hash = input.first()?.clone().into_fixed_bytes()?;
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

#[cfg(test)]
mod tests {
    use candid::Principal;

    use super::*;
    use crate::H256;

    #[test]
    fn notification_transaction_roundtrip() {
        let data = NotificationInput {
            about_tx: Some(H256::from([1; 32])),
            receiver_canister: Principal::management_canister(),
            user_data: vec![1, 2, 3, 4, 5],
        };

        let encoded = data.clone().encode().unwrap();
        let decoded = NotificationInput::decode(&encoded).unwrap();

        assert_eq!(decoded, data)
    }

    #[test]
    fn invalid_principal() {
        // data length is too big
        let principal_data = vec![42; 32];
        let encoded = NOTIFICATION
            .encode_input(&[
                Token::FixedBytes(H256::from([1; 32]).0 .0.into()),
                Token::FixedBytes(principal_data),
                Token::Bytes(vec![]),
            ])
            .unwrap();
        let decoded = NotificationInput::decode(&encoded);

        assert_eq!(decoded, None);
    }
}
