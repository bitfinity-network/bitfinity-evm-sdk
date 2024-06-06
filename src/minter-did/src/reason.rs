use candid::{CandidType, Principal};
use did::keccak::keccak_hash;
use did::transaction::Signature;
use did::{H160, H256, U256};
use ic_exports::icrc_types::icrc1::account::Subaccount;
use ic_stable_structures::Storable;
use serde::Deserialize;

/// Information to perform burn operation for ICRC-2 token and create a mint order.
#[derive(Debug, Clone, Deserialize, CandidType)]
pub struct Icrc2Burn {
    /// Amount to burn;
    pub amount: U256,

    /// Principal of ICRC-2 token to burn.
    pub icrc2_token_principal: Principal,

    /// Subaccount of the ICRC-2 token from which amount will be burned.
    pub from_subaccount: Option<Subaccount>,

    /// Address of the Wrapped token recipient.
    pub recipient_address: H160,

    /// If user want's mint operation to approve minted tokens,
    /// he can use this field.
    pub approve_minted_tokens: Option<ApproveMintedTokens>,

    /// Address from which fee should be charged for mint transaction
    /// performed by minter canister.
    /// If None, mint transaction will not be sent and user can send it by himself.
    pub fee_payer: Option<H160>,
}

#[derive(Debug, Clone, Deserialize, CandidType)]
pub struct ApproveMintedTokens {
    /// Approve minted tokens for this address.
    pub approve_spender: H160,

    /// Approve minted tokens amount.
    pub approve_amount: U256,

    pub chain_id: u32,
    /// Expiration time in seconds since Unix epoch.
    pub expiration: u64,
    /// Nonce of the sender
    pub nonce: u32,
    pub token_principal: Principal,

    /// Signed keccak256 hash of [ApproveMintSignature].
    /// Required to prove caller's ownership of the wallet from which the minted tokens will be approved.
    pub signature: Signature,
}

impl ApproveMintedTokens {
    pub fn new(
        approve_spender: H160,
        approve_amount: U256,
        chain_id: u32,
        expiration: u64,
        nonce: u32,
        token_principal: Principal,
        signature: Signature,
    ) -> Self {
        Self {
            approve_spender,
            approve_amount,
            chain_id,
            expiration,
            nonce,
            token_principal,
            signature,
        }
    }

    /// Hash of the signature data to be signed.
    pub fn hash(
        approve_spender: &H160,
        approve_amount: &U256,
        chain_id: u32,
        expiration: u64,
        nonce: u32,
        token_principal: Principal,
    ) -> H256 {
        let mut data = Vec::new();
        data.extend_from_slice(&approve_spender.to_bytes());
        data.extend_from_slice(&approve_amount.to_little_endian());
        data.extend_from_slice(&chain_id.to_le_bytes());
        data.extend_from_slice(&expiration.to_le_bytes());
        data.extend_from_slice(&nonce.to_le_bytes());
        data.extend_from_slice(token_principal.as_slice());

        keccak_hash(data.as_slice())
    }

    pub fn check_signature(&self, signer: &H160) -> Option<()> {
        let eth_signature: ethers_core::types::Signature = self.signature.clone().into();
        let hash = Self::hash(
            &self.approve_spender,
            &self.approve_amount,
            self.chain_id,
            self.expiration,
            self.nonce,
            self.token_principal,
        );
        let recovered_signer = eth_signature.recover(hash.0).ok()?;
        if recovered_signer != signer.0 {
            return None;
        }

        Some(())
    }

    /// Validate the nonce and expiration time.
    ///
    /// This is to prevent replay attacks.
    pub fn validate(&self, expected_nonce: u32, time: u64) -> crate::error::Result<()> {
        if self.expiration < time {
            return Err(crate::error::Error::Internal(
                format!("approve error: the approve expired at ts: {}", time).to_string(),
            ));
        }

        if self.nonce != expected_nonce {
            return Err(crate::error::Error::Internal(
                format!(
                    "approve error: invalid nonce: expected {}, got {}",
                    expected_nonce, self.nonce
                )
                .to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use candid::Principal;
    use did::{H160, U256};
    use eth_signer::{Signer, Wallet};

    use super::ApproveMintedTokens;
    #[test]
    fn should_accept_correct_signature() {
        let token_principal = Principal::from_slice(&[42; 20]);
        let approve_spender = H160::default();
        let approve_amount = U256::default();

        let approve_signature_data =
            ApproveMintedTokens::hash(&approve_spender, &approve_amount, 1, 0, 0, token_principal);

        let wallet = Wallet::new(&mut rand::thread_rng());
        let signature = wallet.sign_hash(approve_signature_data.0).unwrap();

        let approve_data = ApproveMintedTokens::new(
            approve_spender,
            approve_amount,
            1,
            0,
            0,
            token_principal,
            signature.into(),
        );

        assert!(approve_data
            .check_signature(&wallet.address().into())
            .is_some());
    }

    #[test]
    fn should_reject_invalid_signature() {
        let approve_signature_hash = ApproveMintedTokens::hash(
            &H160::default(),
            &U256::default(),
            1,
            0,
            0,
            Principal::from_slice(&[42; 20]),
        );

        let wallet = Wallet::new(&mut rand::thread_rng());
        let signature = wallet.sign_hash(approve_signature_hash.0).unwrap();

        let approve_data = ApproveMintedTokens::new(
            H160::default(),
            U256::default(),
            1,
            0,
            0,
            Principal::from_slice(&[43; 20]),
            signature.into(),
        );

        assert!(approve_data
            .check_signature(&wallet.address().into())
            .is_none());
    }

    #[test]
    fn validate_nonce_and_expiration() {
        let data = ApproveMintedTokens::new(
            H160::default(),
            U256::default(),
            1,
            50,
            0,
            Principal::from_slice(&[42; 20]),
            did::transaction::Signature::default(),
        );

        let validate = data.validate(0, 100);

        assert!(validate.is_err());

        let err = validate.unwrap_err();
        assert_eq!(
            err.to_string(),
            "internal error: approve error: the approve expired at ts: 100"
        );

        // nonce is invalid
        let validate = data.validate(1, 49);
        assert!(validate.is_err());

        //valid data
        let validate = data.validate(0, 49);
        assert!(validate.is_ok());
    }
}
