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

    /// Signed keccak256 hash of [ApproveMintSignature].
    /// Required to prove caller's ownership of the wallet from which the minted tokens will be approved.
    pub signature: Signature,
}

/// Represents the signature data required to approve minted tokens.
///
pub struct ApproveMintSignature {
    pub approve_spender: H160,
    pub approve_amount: U256,
    pub chain_id: u32,
    /// Expiration time in seconds since Unix epoch.
    pub expiration: u64,
    /// Nonce of the sender
    pub nonce: u32,
    pub token_principal: Principal,
}

impl ApproveMintSignature {
    /// Creates a new signature
    pub fn new(
        approve_spender: H160,
        approve_amount: U256,
        nonce: u32,
        expiration: u64,
        token_principal: Principal,
        chain_id: u32,
    ) -> Self {
        Self {
            approve_spender,
            approve_amount,
            nonce,
            expiration,
            token_principal,
            chain_id,
        }
    }

    /// Hash of the signature data to be signed.
    pub fn hash(&self) -> H256 {
        let mut data = Vec::new();
        data.extend_from_slice(&self.approve_spender.to_bytes());
        data.extend_from_slice(&self.approve_amount.to_little_endian());
        data.extend_from_slice(&self.chain_id.to_le_bytes());
        data.extend_from_slice(&self.expiration.to_le_bytes());
        data.extend_from_slice(&self.nonce.to_le_bytes());
        data.extend_from_slice(self.token_principal.as_slice());

        keccak_hash(data.as_slice())
    }

    pub fn check_signature(&self, signer: &H160, signature: Signature) -> Option<()> {
        let eth_signature: ethers_core::types::Signature = signature.clone().into();
        let hash = self.hash();
        let recovered_signer = eth_signature.recover(hash.0).ok()?;
        if recovered_signer != signer.0 {
            return None;
        }

        Some(())
    }
}

#[cfg(test)]
mod tests {
    use candid::Principal;
    use did::keccak::keccak_hash;
    use did::{H160, U256};
    use eth_signer::{Signer, Wallet};

    use super::ApproveMintSignature;
    use super::ApproveMintedTokens;
    #[test]
    fn should_accept_correct_signature() {
        let principal = Principal::from_slice(&[42; 20]);
        let approve_signature_data =
            ApproveMintSignature::new(H160::default(), U256::default(), 0, 0, principal, 0);

        let wallet = Wallet::new(&mut rand::thread_rng());
        let signature = wallet.sign_hash(approve_signature_data.hash().0).unwrap();

        assert!(approve_signature_data
            .check_signature(&wallet.address().into(), signature.into())
            .is_some());
    }

    #[test]
    fn should_reject_invalid_signature() {
        let principal = Principal::from_slice(&[42; 20]);
        let mut approve_signature_data =
            ApproveMintSignature::new(H160::default(), U256::default(), 0, 0, principal, 0);
        let wallet = Wallet::new(&mut rand::thread_rng());
        let signature = wallet.sign_hash(approve_signature_data.hash().0).unwrap();

        approve_signature_data.token_principal = Principal::from_slice(&[43; 20]);
        assert!(approve_signature_data
            .check_signature(&wallet.address().into(), signature.into())
            .is_none());
    }
}
