use std::fmt;

use alloy::consensus::SignableTransaction;
use alloy::primitives::{Address, PrimitiveSignature, SignatureError, U256};
use alloy::signers::k256::ecdsa::{RecoveryId, Signature as EcsaSignature, VerifyingKey};
use alloy::signers::utils::public_key_to_address;
use candid::{CandidType, Principal};
use did::transaction::Signature as DidSignature;
use ic_canister::virtual_canister_call;
use ic_exports::ic_cdk::api::call::RejectionCode;
use ic_exports::ic_cdk::api::management_canister::ecdsa::{
    EcdsaCurve, EcdsaKeyId, EcdsaPublicKeyArgument, EcdsaPublicKeyResponse, SignWithEcdsaArgument,
    SignWithEcdsaResponse,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub type DerivationPath = Vec<Vec<u8>>;

#[derive(Debug, Error)]
pub enum IcSignerError {
    #[error("IC failed to sign data with rejection code {0:?}: {1}")]
    SigningFailed(RejectionCode, String),

    #[error("from address is not specified in transaction")]
    FromAddressNotPresent,

    #[error("invalid public key")]
    InvalidPublicKey,

    #[error(transparent)]
    SignatureError(#[from] SignatureError),

    #[error("internal error occurred : {0}")]
    Internal(String),
}

/// Signing key which will be used by management canister.
#[derive(Debug, Clone, Serialize, Deserialize, CandidType, PartialEq, Eq)]
pub enum SigningKeyId {
    /// A default key ID that is used in deploying to a local version of IC (via DFX).
    Dfx,

    /// A master test key ID that is used on the mainnet.
    Test,

    /// A master production key ID that is used on the mainnet.
    Production,

    /// A key ID available in the Pocket IC server
    PocketIc,

    /// A key ID that is not defined in the enum
    Custom(String),
}

/// There are three key options:
/// - dfx_test_key: a default key ID that is used in deploying to a local version of IC (via IC SDK).
/// - test_key_1: a master test key ID that is used on the mainnet.
/// - key_1: a master production key ID that is used on the mainnet.
///
/// Source: https://internetcomputer.org/docs/current/samples/t-ecdsa-sample#update-source-code-with-the-right-key-id
impl fmt::Display for SigningKeyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SigningKeyId::Dfx => write!(f, "dfx_test_key"),
            SigningKeyId::Test => write!(f, "test_key_1"),
            SigningKeyId::Production => write!(f, "key_1"),
            SigningKeyId::PocketIc => {
                write!(f, "dfx_test_key")
            }
            SigningKeyId::Custom(key) => write!(f, "{}", key),
        }
    }
}

#[derive(Default)]
pub struct IcSigner;

impl IcSigner {
    /// Compute the recovery ID for the given digest, public key and signature.
    ///
    /// # Arguments
    /// * `digest`: The digest to recover the signature from.
    /// * `pubkey`: The public key that was used to sign the digest.
    /// * `signature`: The signature to recover.
    ///
    /// # Returns
    /// The recovery ID.
    pub fn compute_recovery_id(
        digest: &[u8],
        pubkey: &[u8],
        signature: &[u8],
    ) -> Result<RecoveryId, IcSignerError> {
        let pub_key =
            VerifyingKey::from_sec1_bytes(pubkey).map_err(|_| IcSignerError::InvalidPublicKey)?;

        let sec_signature = EcsaSignature::from_slice(signature).map_err(|e| {
            IcSignerError::Internal(format!("failed to parse ECDSA signature: {e}"))
        })?;

        let recovery_id = RecoveryId::trial_recovery_from_prehash(&pub_key, digest, &sec_signature)
            .map_err(|e| IcSignerError::Internal(format!("failed to compute recovery ID: {e}")))?;

        Ok(recovery_id)
    }

    /// Signs the transaction using `ManagementCanister::sign_with_ecdsa()`
    /// call.
    pub async fn sign_transaction(
        &self,
        tx: &mut dyn SignableTransaction<PrimitiveSignature>,
        pubkey: &[u8],
        key_id: SigningKeyId,
        derivation_path: DerivationPath,
    ) -> Result<DidSignature, IcSignerError> {
        let hash = tx.signature_hash();
        let mut signature = Self
            .sign_digest(*hash, pubkey, key_id, derivation_path)
            .await?;

        let v: u64 = signature.v.0.to();

        // For non-legacy transactions recovery id should be updated.
        // Details: https://eips.ethereum.org/EIPS/eip-155.
        signature.v = match tx.chain_id() {
            Some(chain_id) => (chain_id * 2 + 35 + (v - 27)).into(),
            None => v.into(),
        };

        Ok(signature)
    }

    /// Signs the digest using `ManagementCanister::sign_with_ecdsa()` call.
    pub async fn sign_digest(
        &self,
        digest: [u8; 32],
        pub_key: &[u8],
        key_id: SigningKeyId,
        derivation_path: DerivationPath,
    ) -> Result<DidSignature, IcSignerError> {
        let request = SignWithEcdsaArgument {
            key_id: EcdsaKeyId {
                curve: EcdsaCurve::Secp256k1,
                name: key_id.to_string(),
            },
            message_hash: digest.to_vec(),
            derivation_path: derivation_path.clone(),
        };

        let signature_data = virtual_canister_call!(
            Principal::management_canister(),
            "sign_with_ecdsa",
            (request,),
            SignWithEcdsaResponse,
            100_000_000_000
        )
        .await
        .map_err(|(code, msg)| IcSignerError::SigningFailed(code, msg))?
        .signature;

        // IC doesn't support recovery id signature parameter, so we
        // use the public key with the signature to compute the recovery id.
        // Details: https://eips.ethereum.org/EIPS/eip-155.
        let recovery_id = Self::compute_recovery_id(&digest, pub_key, &signature_data)?;

        let r = U256::from_be_slice(&signature_data[0..32]);
        let s = U256::from_be_slice(&signature_data[32..64]);
        let v = recovery_id.is_y_odd() as u64 + 27;

        // Signature malleability check is not required, because dfinity uses `k256` crate
        // as `ecdsa_secp256k1` implementation, and it takes care about signature malleability.
        // Link: https://github.com/dfinity/ic/blob/master/rs/crypto/ecdsa_secp256k1/src/lib.rs
        let signature = DidSignature { r: r.into(), s: s.into(), v: v.into() };

        Ok(signature)
    }

    /// Returns public key for current canister from IC.
    pub async fn public_key(
        &self,
        key_id: SigningKeyId,
        derivation_path: DerivationPath,
    ) -> Result<Vec<u8>, IcSignerError> {
        let request = EcdsaPublicKeyArgument {
            canister_id: None,
            derivation_path,
            key_id: EcdsaKeyId {
                curve: EcdsaCurve::Secp256k1,
                name: key_id.to_string(),
            },
        };
        virtual_canister_call!(
            Principal::management_canister(),
            "ecdsa_public_key",
            (request,),
            EcdsaPublicKeyResponse
        )
        .await
        .map_err(|(code, msg)| IcSignerError::SigningFailed(code, msg))
        .map(|response| response.public_key)
    }

    /// Convert public key to ethereum address.
    pub fn pubkey_to_address(&self, pubkey: &[u8]) -> Result<Address, IcSignerError> {
        let key =
            VerifyingKey::from_sec1_bytes(pubkey).map_err(|_| IcSignerError::InvalidPublicKey)?;

        Ok(public_key_to_address(&key))
    }
}

#[cfg(test)]
mod tests {

    use alloy::network::TransactionBuilder;
    use alloy::primitives::{B160, B256};
    use alloy::rpc::types::TransactionRequest;
    use alloy::signers::k256::ecdsa::signature;
    use alloy::signers::SignerSync;
    use candid::Principal;
    use did::H256;
    use ic_canister::register_virtual_responder;
    use ic_exports::ic_cdk::api::management_canister::ecdsa::{
        EcdsaPublicKeyArgument, EcdsaPublicKeyResponse, SignWithEcdsaArgument,
        SignWithEcdsaResponse,
    };
    use ic_exports::ic_kit::MockContext;

    use super::{IcSigner, *};
    use crate::ic_sign::SigningKeyId;
    use crate::LocalWallet;

    fn init_context() -> LocalWallet {
        MockContext::new().inject();

        let wallet = LocalWallet::random_with(&mut rand::thread_rng());
        let pubkey = wallet.credential().verifying_key().to_encoded_point(true);

        let wallet_to_sign = wallet.clone();
        register_virtual_responder(
            Principal::management_canister(),
            "sign_with_ecdsa",
            move |args: (SignWithEcdsaArgument,)| {
                let hash = args.0.message_hash;
                let h256 = H256::from_slice(&hash);
                let signature = wallet_to_sign.sign_hash_sync(&h256.0).unwrap();
                let signature: Vec<u8> = signature.as_bytes().into_iter().take(64).collect();
                SignWithEcdsaResponse { signature }
            },
        );

        register_virtual_responder(
            Principal::management_canister(),
            "ecdsa_public_key",
            move |_: (EcdsaPublicKeyArgument,)| EcdsaPublicKeyResponse {
                public_key: pubkey.as_bytes().to_vec(),
                chain_code: vec![],
            },
        );

        wallet
    }

    #[tokio::test]
    async fn should_sign_transactions() {
        let wallet = init_context();
        let from = wallet.address();
        let tx = TransactionRequest::default()
            .with_from(from)
            .with_to(Address::ZERO)
            .with_value(U256::from(10u64))
            .with_chain_id(355113)
            .with_nonce(0)
            .with_gas_price(10)
            .with_gas_limit(53000)
            .build_consensus_tx().unwrap();
        let mut tx = tx.legacy().cloned().unwrap();

        let pub_key = wallet.credential().verifying_key().to_encoded_point(true);

        let signature = IcSigner
            .sign_transaction(
                &mut tx,
                &pub_key.to_bytes(),
                SigningKeyId::Dfx,
                DerivationPath::default(),
            )
            .await
            .unwrap();

        let primitive_signature = alloy::primitives::PrimitiveSignature::try_from(signature).unwrap();

        let recovered_from = primitive_signature.recover_address_from_prehash(&tx.signature_hash()).unwrap();
        assert_eq!(recovered_from, from);
    }

    #[test]
    fn test_pubkey_to_address() {
        let wallet = init_context();
        let pubkey = wallet.credential().verifying_key().to_encoded_point(true);
        let address = IcSigner.pubkey_to_address(&pubkey.to_bytes()).unwrap();
        assert_eq!(address, wallet.address());
    }
}
