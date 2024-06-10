use candid::{CandidType, Principal};
use did::keccak::keccak_hash;
use did::transaction::Signature;
use did::{H160, U256};
use ic_exports::icrc_types::icrc1::account::Subaccount;
use serde::{Deserialize, Serialize};

/// Information to perform burn operation for ICRC-2 token and create a mint order.
#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
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

#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct ApproveMintedTokens {
    /// Approve minted tokens for this address.
    pub approve_spender: H160,

    /// Approve minted tokens amount.
    pub approve_amount: U256,

    /// Signed `keccak_hash(caller's principal)` by the recipient wallet.
    /// Reqired to prove caller's ownership of the wallet from which the minted tokens will be approved.
    pub principal_signature: Signature,
}

impl ApproveMintedTokens {
    /// Checks if `self.principal_signature` is correct for the given `principal` and `signer`.
    pub fn check_signature(&self, principal: &Principal, signer: &H160) -> Option<()> {
        let eth_signature: ethers_core::types::Signature = self.principal_signature.clone().into();
        let principal_hash = keccak_hash(principal.as_slice());
        let recovered_signer = eth_signature.recover(principal_hash.0).ok()?;
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

    use super::ApproveMintedTokens;

    #[test]
    fn should_accept_correct_signature() {
        let principal = Principal::from_slice(&[42; 20]);
        let principal_hash = keccak_hash(principal.as_slice());

        let wallet = Wallet::new(&mut rand::thread_rng());
        let signature = wallet.sign_hash(principal_hash.0).unwrap();

        let approve = ApproveMintedTokens {
            approve_spender: H160::default(),
            approve_amount: U256::default(),
            principal_signature: signature.into(),
        };

        assert!(approve
            .check_signature(&principal, &wallet.address().into())
            .is_some());
    }

    #[test]
    fn should_reject_invalid_signature() {
        let principal = Principal::from_slice(&[42; 20]);
        let principal_hash = keccak_hash(principal.as_slice());

        let wallet = Wallet::new(&mut rand::thread_rng());
        let signature = wallet.sign_hash(principal_hash.0).unwrap();

        let approve = ApproveMintedTokens {
            approve_spender: H160::default(),
            approve_amount: U256::default(),
            principal_signature: signature.into(),
        };

        let other_principal = Principal::from_slice(&[122; 20]);
        assert!(approve
            .check_signature(&other_principal, &wallet.address().into())
            .is_none());
    }
}
