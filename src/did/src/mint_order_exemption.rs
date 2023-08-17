use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};

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
        let user = Principal::try_from_slice(&tx_input[4..]).ok()?;

        Some(Self { user, weeks })
    }
}

#[cfg(test)]
mod tests {

    use candid::Principal;
    use ethers_core::abi::Token;

    use super::*;
    use crate::notify::{NotificationInput, NOTIFICATION};
    use crate::H256;

    #[test]
    fn mint_order_exemption_encoding() {
        let user_principal = Principal::anonymous();

        let mut user_data: Vec<u8> = Vec::with_capacity(NotificationInput::MIN_INPUT_LEN);
        user_data.extend_from_slice(&4_u32.to_le_bytes());
        user_data.extend_from_slice(user_principal.as_slice());

        let data = NotificationInput {
            about_tx: Some(H256::from([1; 32])),
            receiver_canister: Principal::management_canister(),
            user_data,
        };

        let encoded = data.clone().encode().unwrap();
        let decoded = NotificationInput::decode(&encoded).unwrap();

        let user_data = MintOrderExemptionUserData::decode(&decoded.user_data).unwrap();
        assert_eq!(user_data.user, user_principal);
        assert_eq!(user_data.weeks, 4);

        assert_eq!(decoded, data)
    }

    #[test]
    fn invalid_principal() {
        // data length is too big
        let principal_data = vec![64; 32];
        let encoded = NOTIFICATION
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
