use std::fmt;

use candid::{CandidType, Principal};
use ethers_core::k256::elliptic_curve::sec1::ToEncodedPoint;
use ethers_core::k256::PublicKey;
use ethers_core::types::transaction::eip2718::TypedTransaction;
use ethers_core::types::{Signature, SignatureError, H160};
use ethers_core::utils;
use ic_canister::virtual_canister_call;
use ic_exports::ic_ic00_types::{
    DerivationPath, ECDSAPublicKeyArgs, ECDSAPublicKeyResponse, EcdsaCurve, EcdsaKeyId,
    SignWithECDSAArgs,
};
use ic_exports::ic_kit::RejectionCode;
use ic_exports::serde::{Deserialize, Serialize};
use thiserror::Error;

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
}

/// Signing key which will be used by management canister.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, CandidType, PartialEq, Eq)]
pub enum SigningKeyId {
    /// A default key ID that is used in deploying to a local version of IC (via DFX).
    Dfx,

    /// A master test key ID that is used on the mainnet.
    Test,

    /// A master production key ID that is used on the mainnet.
    Production,
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
        }
    }
}

#[derive(Default)]
pub struct IcSigner;

impl IcSigner {
    /// Signs the transaction using `ManagementCanister::sign_with_ecdsa()` call.
    /// The `tx.from` expected to be set to the canister address.
    pub async fn sign_transaction(
        &self,
        tx: &TypedTransaction,
        key_id: SigningKeyId,
        derivation_path: DerivationPath,
    ) -> Result<Signature, IcSignerError> {
        let hash = tx.sighash();
        let digest = hash.as_fixed_bytes();
        let tx_from = tx.from().ok_or(IcSignerError::FromAddressNotPresent)?;
        let mut signature = Self
            .sign_digest(tx_from, *digest, key_id, derivation_path)
            .await?;

        // For non-legacy transactions recovery id should be updated.
        // Details: https://eips.ethereum.org/EIPS/eip-155.
        signature.v += match &tx {
            TypedTransaction::Legacy(_) => 27,
            _ => tx.chain_id().unwrap_or_default().as_u64() * 2 + 35,
        };

        Ok(signature)
    }

    /// Signs the digest using `ManagementCanister::sign_with_ecdsa()` call.
    pub async fn sign_digest(
        &self,
        canister_address: &H160,
        digest: [u8; 32],
        key_id: SigningKeyId,
        derivation_path: DerivationPath,
    ) -> Result<Signature, IcSignerError> {
        let request = SignWithECDSAArgs {
            key_id: EcdsaKeyId {
                curve: EcdsaCurve::Secp256k1,
                name: key_id.to_string(),
            },
            message_hash: digest,
            derivation_path,
        };
        let signature_data = virtual_canister_call!(
            Principal::management_canister(),
            "sign_with_ecdsa",
            (request,),
            ic_exports::ic_ic00_types::SignWithECDSAReply
        )
        .await
        .map_err(|(code, msg)| IcSignerError::SigningFailed(code, msg))?
        .signature;

        let r = ethers_core::types::U256::from_big_endian(&signature_data[0..32]);
        let s = ethers_core::types::U256::from_big_endian(&signature_data[32..64]);

        // IC doesn't support recovery id signature parameter, so set it manually.
        // Details: https://eips.ethereum.org/EIPS/eip-155.
        let mut signature = Signature { r, s, v: 0 };

        // Recovery id value may be increased by one, depending on internal
        // signing parameter we don't know.
        // The only thing we can do: try to recover address and, if failed,
        // assume that recovery id should be increased.
        let recovered = signature.recover(digest)?;
        if &recovered != canister_address {
            signature.v += 1;
        };

        Ok(signature)
    }

    /// Returns public key for current canister from IC.
    pub async fn public_key(
        &self,
        key_id: SigningKeyId,
        derivation_path: DerivationPath,
    ) -> Result<Vec<u8>, IcSignerError> {
        let request = ECDSAPublicKeyArgs {
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
            ECDSAPublicKeyResponse
        )
        .await
        .map_err(|(code, msg)| IcSignerError::SigningFailed(code, msg))
        .map(|response| response.public_key)
    }

    /// Convert public key to ethereum address.
    pub fn pubkey_to_address(&self, pubkey: &[u8]) -> Result<H160, IcSignerError> {
        let uncompressed_public_key =
            PublicKey::from_sec1_bytes(pubkey).map_err(|_| IcSignerError::InvalidPublicKey)?;

        let public_key = uncompressed_public_key.to_encoded_point(false);
        let public_key = public_key.as_bytes();
        debug_assert_eq!(public_key[0], 0x04);
        let hash = utils::keccak256(&public_key[1..]);

        let mut bytes = [0u8; 20];
        bytes.copy_from_slice(&hash[12..]);
        Ok(H160::from_slice(&bytes))
    }
}

#[cfg(test)]
mod tests {
    use candid::Principal;
    use ethers_core::k256::ecdsa::SigningKey;
    use ethers_core::types::transaction::eip2718::TypedTransaction;
    use ethers_core::types::{TransactionRequest, H160, H256};
    use ic_canister::register_virtual_responder;
    use ic_exports::ic_ic00_types::{
        DerivationPath, ECDSAPublicKeyArgs, ECDSAPublicKeyResponse, SignWithECDSAArgs,
        SignWithECDSAReply,
    };
    use ic_exports::ic_kit::MockContext;

    use super::IcSigner;
    use crate::ic_sign::SigningKeyId;
    use crate::Wallet;

    fn init_context() -> Wallet<'static, SigningKey> {
        MockContext::new().inject();

        let wallet = Wallet::new(&mut rand::thread_rng());
        let pubkey = wallet.signer.verifying_key().to_encoded_point(true);

        let wallet_to_sign = wallet.clone();
        register_virtual_responder(
            Principal::management_canister(),
            "sign_with_ecdsa",
            move |args: (SignWithECDSAArgs,)| {
                let hash = args.0.message_hash;
                let h256 = H256::from_slice(&hash);
                let signature = wallet_to_sign.sign_hash(h256).unwrap();
                SignWithECDSAReply {
                    signature: signature.to_vec(),
                }
            },
        );

        register_virtual_responder(
            Principal::management_canister(),
            "ecdsa_public_key",
            move |_: (ECDSAPublicKeyArgs,)| ECDSAPublicKeyResponse {
                public_key: pubkey.as_bytes().to_vec(),
                chain_code: vec![],
            },
        );

        wallet
    }

    #[tokio::test]
    async fn should_sign_transactions() {
        let wallet = init_context();
        let from = wallet.address;
        let tx: TypedTransaction = TransactionRequest::new()
            .from(from)
            .to(H160::zero())
            .value(10)
            .chain_id(355113)
            .nonce(0)
            .gas_price(10)
            .gas(53000)
            .into();

        let signature = IcSigner
            .sign_transaction(&tx, SigningKeyId::Dfx, DerivationPath::new(vec![]))
            .await
            .unwrap();

        let recovered_from = signature.recover(tx.sighash()).unwrap();
        assert_eq!(recovered_from, from);
    }
}
