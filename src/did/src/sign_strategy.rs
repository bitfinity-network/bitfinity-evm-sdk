use std::borrow::Cow;
use std::cell::RefCell;

use async_trait::async_trait;
use candid::CandidType;
pub use eth_signer::ic_sign::SigningKeyId;
use eth_signer::ic_sign::{DerivationPath, IcSigner};
use eth_signer::{Signer, Wallet};
use ethers_core::k256::ecdsa::SigningKey;
use ethers_core::types::transaction::eip2718::TypedTransaction;
use ethers_core::utils;
use ic_stable_structures::Storable;
use serde::{Deserialize, Serialize};

use crate::error::{EvmError, Result};
use crate::transaction::Signature;
use crate::H160;

/// A trait that abstracts out the transaction signing component
#[async_trait(?Send)]
pub trait TransactionSigner {
    /// Returns the `sender` address for the given identity
    async fn get_address(&self) -> Result<H160>;

    /// Sign the created transaction
    async fn sign_transaction(&self, transaction: &TypedTransaction) -> Result<Signature>;

    /// Sign the given digest
    async fn sign_digest(&self, digest: [u8; 32]) -> Result<Signature>;
}

/// Signing strategy for signing EVM transactions
#[derive(CandidType, Clone, Debug, Deserialize, PartialEq, Eq)]
pub enum SigningStrategy {
    /// Local signing key
    Local { private_key: [u8; 32] },
    /// Use management canister and ECDSA signing endpoints
    ManagementCanister { key_id: SigningKeyId },
}

impl SigningStrategy {
    /// Create signing object from the current strategy
    pub fn make_signer(self, chain_id: u64) -> Result<TxSigner> {
        match self {
            SigningStrategy::Local { private_key } => {
                let signer = SigningKey::from_slice(&private_key).map_err(|e| {
                    EvmError::from(format!("failed to deserialize signing key: {e}"))
                })?;
                let address = utils::secret_key_to_address(&signer);
                let wallet = Wallet::new_with_signer(Cow::Owned(signer), address, chain_id);
                Ok(TxSigner::Local(LocalTxSigner::new(wallet)))
            }
            SigningStrategy::ManagementCanister { key_id } => {
                let derivation_path = vec![chain_id.to_be_bytes().to_vec()];
                Ok(TxSigner::ManagementCanister(ManagementCanisterSigner::new(
                    key_id,
                    derivation_path,
                )))
            }
        }
    }
}

/// Transaction signer
#[derive(Serialize, Deserialize, Clone)]
pub enum TxSigner {
    Local(LocalTxSigner),
    ManagementCanister(ManagementCanisterSigner),
}

impl Storable for TxSigner {
    fn to_bytes(&self) -> Cow<[u8]> {
        bincode::serialize(self)
            .expect("failed to serialize TxSigner")
            .into()
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        bincode::deserialize(&bytes).expect("failed to deserialize TxSigner")
    }
}

#[async_trait(?Send)]
impl TransactionSigner for TxSigner {
    async fn get_address(&self) -> Result<H160> {
        match self {
            Self::Local(signer) => signer.get_address().await,
            Self::ManagementCanister(signer) => signer.get_address().await,
        }
    }

    async fn sign_transaction(&self, transaction: &TypedTransaction) -> Result<Signature> {
        match self {
            Self::Local(signer) => signer.sign_transaction(transaction).await,
            Self::ManagementCanister(signer) => signer.sign_transaction(transaction).await,
        }
    }

    async fn sign_digest(&self, digest: [u8; 32]) -> Result<Signature> {
        match self {
            Self::Local(signer) => signer.sign_digest(digest).await,
            Self::ManagementCanister(signer) => signer.sign_digest(digest).await,
        }
    }
}

/// Local private key implementation
#[derive(Clone)]
pub struct LocalTxSigner {
    wallet: Wallet<'static, SigningKey>,
}

impl LocalTxSigner {
    fn new(wallet: Wallet<'static, SigningKey>) -> LocalTxSigner {
        Self { wallet }
    }
}

#[async_trait(?Send)]
impl TransactionSigner for LocalTxSigner {
    async fn get_address(&self) -> Result<H160> {
        Ok(self.wallet.address().into())
    }

    async fn sign_transaction(&self, transaction: &TypedTransaction) -> Result<Signature> {
        self.wallet
            .sign_transaction(transaction)
            .await
            .map_err(|e| EvmError::from(format!("failed to sign hash: {e}")))
            .map(Into::into)
    }

    async fn sign_digest(&self, digest: [u8; 32]) -> Result<Signature> {
        self.wallet
            .sign_hash(ethereum_types::H256(digest))
            .map_err(|e| EvmError::from(format!("failed to sign hash: {e}")))
            .map(Into::into)
    }
}

/// A helper struct for serializing/deserializing `LocalTxSigner`
#[derive(Serialize, Deserialize)]
struct WalletSerializationData<'a> {
    signing_key_bytes: &'a [u8],
    address_bytes: &'a [u8],
    chain_id: u64,
}

impl Serialize for LocalTxSigner {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let signing_key_bytes = self.wallet.signer().to_bytes();
        let address = self.wallet.address();
        let chain_id = self.wallet.chain_id();
        let serialization_data = WalletSerializationData {
            signing_key_bytes: &signing_key_bytes,
            address_bytes: address.as_bytes(),
            chain_id,
        };

        serialization_data.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for LocalTxSigner {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;

        let val = WalletSerializationData::deserialize(deserializer)?;
        let signing_key = SigningKey::from_slice(val.signing_key_bytes)
            .map_err(|e| D::Error::custom(format!("failed to decode signing key: {e}")))?;
        let address = H160::from_slice(val.address_bytes);
        Ok(LocalTxSigner::new(Wallet::new_with_signer(
            Cow::Owned(signing_key),
            address.into(),
            val.chain_id,
        )))
    }
}

/// An implementation of a signer that uses Management canister
#[derive(CandidType, Serialize, Deserialize, Clone)]
pub struct ManagementCanisterSigner {
    key_id: SigningKeyId,
    cached_address: RefCell<Option<H160>>,
    derivation_path: DerivationPath,
}

impl ManagementCanisterSigner {
    pub fn new(key_id: SigningKeyId, derivation_path: DerivationPath) -> Self {
        Self {
            key_id,
            cached_address: RefCell::new(None),
            derivation_path,
        }
    }
}

#[async_trait(?Send)]
impl TransactionSigner for ManagementCanisterSigner {
    async fn get_address(&self) -> Result<H160> {
        if let Some(address) = &*self.cached_address.borrow() {
            return Ok(address.clone());
        }

        let pubkey = IcSigner {}
            .public_key(self.key_id, self.derivation_path.clone())
            .await
            .map_err(|e| EvmError::from(format!("failed to get address: {e}")))?;
        let address: H160 = IcSigner
            .pubkey_to_address(&pubkey)
            .map_err(|e| {
                EvmError::Internal(format!("failed to convert public key to address: {e}"))
            })?
            .into();
        *self.cached_address.borrow_mut() = Some(address.clone());

        Ok(address)
    }

    async fn sign_transaction(&self, transaction: &TypedTransaction) -> Result<Signature> {
        IcSigner {}
            .sign_transaction(transaction, self.key_id, self.derivation_path.clone())
            .await
            .map_err(|e| EvmError::from(format!("failed to get message signature: {e}")))
            .map(Into::into)
    }

    async fn sign_digest(&self, digest: [u8; 32]) -> Result<Signature> {
        let address = self.get_address().await?;
        IcSigner {}
            .sign_digest(
                &address.into(),
                digest,
                self.key_id,
                self.derivation_path.clone(),
            )
            .await
            .map_err(|e| EvmError::from(format!("failed to get message signature: {e}")))
            .map(Into::into)
    }
}

#[cfg(test)]
mod test {
    use rand::thread_rng;

    use super::*;

    fn storable_roundtrip<T: Storable>(value: &impl Storable) -> T {
        T::from_bytes(value.to_bytes())
    }

    #[test]
    fn test_local_signer_storable_roundtrip() {
        let wallet = Wallet::new(&mut thread_rng());
        let signer = TxSigner::Local(LocalTxSigner {
            wallet: wallet.clone(),
        });
        let signer: TxSigner = storable_roundtrip(&signer);
        if let TxSigner::Local(LocalTxSigner {
            wallet: wallet_roundtrip,
        }) = signer
        {
            assert_eq!(wallet.address(), wallet_roundtrip.address());
            assert_eq!(wallet.signer(), wallet_roundtrip.signer());
            assert_eq!(wallet.chain_id(), wallet_roundtrip.chain_id());
        } else {
            panic!("roundtrip failed");
        }
    }

    #[test]
    fn test_management_canister_signer_roundtrip() {
        let management_canister_signer = ManagementCanisterSigner {
            key_id: SigningKeyId::Dfx,
            cached_address: RefCell::new(Some(H160::from_slice(&[3; 20]))),
            derivation_path: vec![vec![1, 2], vec![3]],
        };
        let signer: TxSigner = storable_roundtrip(&TxSigner::ManagementCanister(
            management_canister_signer.clone(),
        ));
        if let TxSigner::ManagementCanister(ManagementCanisterSigner {
            key_id,
            cached_address,
            derivation_path,
        }) = signer
        {
            assert!(matches!(key_id, SigningKeyId::Dfx));
            assert_eq!(cached_address, management_canister_signer.cached_address);
            assert_eq!(derivation_path, management_canister_signer.derivation_path);
        } else {
            panic!("roundtrip failed");
        }
    }

    #[test]
    fn test_create_local_signer() {
        let signing_strategy = SigningStrategy::Local {
            private_key: [2; 32],
        };
        let signer = signing_strategy.make_signer(42).unwrap();
        if let TxSigner::Local(LocalTxSigner { wallet }) = signer {
            assert_eq!(wallet.chain_id(), 42);
            assert_eq!(wallet.signer().to_bytes().as_slice(), &[2; 32]);
        } else {
            panic!("invalid signer")
        }
    }

    #[test]
    fn test_create_management_signer() {
        let signing_strategy = SigningStrategy::ManagementCanister {
            key_id: SigningKeyId::Test,
        };
        let chain_id = 42;
        let signer = signing_strategy.make_signer(chain_id).unwrap();
        if let TxSigner::ManagementCanister(ManagementCanisterSigner {
            key_id,
            cached_address,
            derivation_path,
        }) = signer
        {
            assert_eq!(key_id, SigningKeyId::Test);
            assert_eq!(derivation_path, vec![chain_id.to_be_bytes().to_vec()]);
            assert_eq!(*cached_address.borrow(), None);
        } else {
            panic!("invalid signer")
        }
    }

    #[tokio::test]
    async fn test_sign_recover() {
        let signing_strategy = SigningStrategy::Local {
            private_key: [2; 32],
        };
        let signer = signing_strategy.make_signer(42).unwrap();
        let digest = [42u8; 32];
        let signature = signer.sign_digest(digest).await.unwrap();
        let recovered = ethers_core::types::Signature::from(signature)
            .recover(digest)
            .unwrap();
        assert_eq!(recovered, signer.get_address().await.unwrap().0);
    }
}
