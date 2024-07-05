use std::borrow::Cow;

use async_trait::async_trait;
use candid::CandidType;
use did::transaction::Signature;
use did::{codec, H160};
use ethers_core::k256::ecdsa::{self, SigningKey};
use ethers_core::types::transaction::eip2718::TypedTransaction;
use ethers_core::utils;
#[cfg(feature = "ic_sign")]
pub use ic_sign::{IcSigner, ManagementCanisterSigner, SigningKeyId};
use ic_stable_structures::{Bound, Storable};
use serde::{Deserialize, Serialize};

use crate::{Signer, Wallet};

#[derive(thiserror::Error, Debug)]
pub enum TransactionSignerError {
    #[error("wallet error: {0}")]
    WalletError(#[from] crate::WalletError),

    #[cfg(feature = "ic_sign")]
    #[error("ic sign error: {0}")]
    IcSignError(#[from] crate::ic_sign::IcSignerError),

    #[error("ecdsa error: {0}")]
    EcdsaError(#[from] ecdsa::Error),
}

pub type TransactionSignerResult<T> = std::result::Result<T, TransactionSignerError>;

/// A trait that abstracts out the transaction signing component
#[async_trait(?Send)]
pub trait TransactionSigner {
    /// Returns the `sender` address for the given identity
    async fn get_address(&self) -> TransactionSignerResult<H160>;

    /// Sign the created transaction
    async fn sign_transaction(
        &self,
        transaction: &TypedTransaction,
    ) -> TransactionSignerResult<Signature>;

    /// Sign the given digest
    async fn sign_digest(&self, digest: [u8; 32]) -> TransactionSignerResult<Signature>;

    /// Get the public key for the given identity
    async fn get_public_key(&self) -> TransactionSignerResult<Vec<u8>>;
}

/// Signing strategy for signing EVM transactions
#[derive(CandidType, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum SigningStrategy {
    /// Local signing key
    Local { private_key: [u8; 32] },
    /// Use management canister and ECDSA signing endpoints
    #[cfg(feature = "ic_sign")]
    ManagementCanister {
        key_id: crate::ic_sign::SigningKeyId,
    },
}

impl SigningStrategy {
    /// Create signing object from the current strategy
    pub fn make_signer(self, chain_id: u64) -> TransactionSignerResult<TxSigner> {
        match self {
            SigningStrategy::Local { private_key } => {
                let signer = SigningKey::from_slice(&private_key)?;
                let address = utils::secret_key_to_address(&signer);
                let wallet = Wallet::new_with_signer(Cow::Owned(signer), address, chain_id);
                Ok(TxSigner::Local(LocalTxSigner::new(wallet)))
            }
            #[cfg(feature = "ic_sign")]
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

impl Storable for SigningStrategy {
    const BOUND: Bound = Bound::Unbounded;

    fn to_bytes(&self) -> Cow<[u8]> {
        codec::bincode_encode(self).into()
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        codec::bincode_decode(&bytes)
    }
}

/// Transaction signer
#[derive(Serialize, Deserialize, Clone)]
pub enum TxSigner {
    Local(LocalTxSigner),
    #[cfg(feature = "ic_sign")]
    ManagementCanister(ManagementCanisterSigner),
}

impl Storable for TxSigner {
    fn to_bytes(&self) -> Cow<[u8]> {
        codec::bincode_encode(self).into()
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        codec::bincode_decode(&bytes)
    }

    const BOUND: Bound = Bound::Unbounded;
}

#[async_trait(?Send)]
impl TransactionSigner for TxSigner {
    async fn get_address(&self) -> TransactionSignerResult<H160> {
        match self {
            Self::Local(signer) => signer.get_address().await,
            #[cfg(feature = "ic_sign")]
            Self::ManagementCanister(signer) => signer.get_address().await,
        }
    }

    async fn sign_transaction(
        &self,
        transaction: &TypedTransaction,
    ) -> TransactionSignerResult<Signature> {
        match self {
            Self::Local(signer) => signer.sign_transaction(transaction).await,
            #[cfg(feature = "ic_sign")]
            Self::ManagementCanister(signer) => signer.sign_transaction(transaction).await,
        }
    }

    async fn sign_digest(&self, digest: [u8; 32]) -> TransactionSignerResult<Signature> {
        match self {
            Self::Local(signer) => signer.sign_digest(digest).await,
            #[cfg(feature = "ic_sign")]
            Self::ManagementCanister(signer) => signer.sign_digest(digest).await,
        }
    }

    async fn get_public_key(&self) -> TransactionSignerResult<Vec<u8>> {
        match self {
            Self::Local(signer) => signer.get_public_key().await,
            #[cfg(feature = "ic_sign")]
            Self::ManagementCanister(signer) => signer.get_public_key().await,
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
    async fn get_address(&self) -> TransactionSignerResult<H160> {
        Ok(self.wallet.address().into())
    }

    async fn sign_transaction(
        &self,
        transaction: &TypedTransaction,
    ) -> TransactionSignerResult<Signature> {
        self.wallet
            .sign_transaction(transaction)
            .await
            .map_err(TransactionSignerError::WalletError)
            .map(Into::into)
    }

    async fn sign_digest(&self, digest: [u8; 32]) -> TransactionSignerResult<Signature> {
        self.wallet
            .sign_hash(ethereum_types::H256(digest))
            .map_err(TransactionSignerError::WalletError)
            .map(Into::into)
    }

    async fn get_public_key(&self) -> TransactionSignerResult<Vec<u8>> {
        Ok(self
            .wallet
            .signer
            .verifying_key()
            .to_encoded_point(false)
            .to_bytes()
            .to_vec())
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

#[cfg(feature = "ic_sign")]
mod ic_sign {
    use std::cell::RefCell;

    use super::*;
    pub use crate::ic_sign::{DerivationPath, IcSigner, SigningKeyId};

    /// An implementation of a signer that uses Management canister
    #[derive(CandidType, Serialize, Deserialize, Clone)]
    pub struct ManagementCanisterSigner {
        pub(super) key_id: SigningKeyId,
        pub(super) derivation_path: DerivationPath,
        pub(super) cached_address: RefCell<Option<H160>>,
        pub(super) cached_pubkey: RefCell<Option<Vec<u8>>>,
    }

    impl ManagementCanisterSigner {
        pub fn new(key_id: SigningKeyId, derivation_path: DerivationPath) -> Self {
            Self {
                key_id,
                derivation_path,
                cached_address: RefCell::new(None),
                cached_pubkey: RefCell::new(None),
            }
        }

        /// Lazily compute the public key
        async fn get_or_compute_pubkey(&self) -> Result<Vec<u8>, TransactionSignerError> {
            let mut cached_pubkey = self.cached_pubkey.borrow_mut();

            if let Some(pubkey) = cached_pubkey.as_ref() {
                return Ok(pubkey.clone());
            }

            let new_pubkey = IcSigner
                .public_key(self.key_id.clone(), self.derivation_path.clone())
                .await?;

            *cached_pubkey = Some(new_pubkey.clone());

            Ok(new_pubkey)
        }
    }

    #[async_trait(?Send)]
    impl TransactionSigner for ManagementCanisterSigner {
        async fn get_address(&self) -> Result<H160, TransactionSignerError> {
            if let Some(address) = self.cached_address.borrow().as_ref() {
                return Ok(address.0.into());
            }

            let pubkey = self.get_or_compute_pubkey().await?;

            let address: H160 = IcSigner.pubkey_to_address(&pubkey)?.into();
            *self.cached_address.borrow_mut() = Some(address.0.into());

            Ok(address)
        }

        async fn sign_transaction(
            &self,
            transaction: &TypedTransaction,
        ) -> Result<Signature, TransactionSignerError> {
            let pub_key = self.get_or_compute_pubkey().await?;

            IcSigner
                .sign_transaction(
                    transaction,
                    &pub_key,
                    self.key_id.clone(),
                    self.derivation_path.clone(),
                )
                .await
                .map_err(TransactionSignerError::IcSignError)
                .map(Into::into)
        }

        async fn sign_digest(&self, digest: [u8; 32]) -> Result<Signature, TransactionSignerError> {
            let pub_key = self.get_or_compute_pubkey().await?;

            IcSigner
                .sign_digest(
                    digest,
                    &pub_key,
                    self.key_id.clone(),
                    self.derivation_path.clone(),
                )
                .await
                .map_err(TransactionSignerError::IcSignError)
                .map(Into::into)
        }

        async fn get_public_key(&self) -> Result<Vec<u8>, TransactionSignerError> {
            self.get_or_compute_pubkey().await
        }
    }
}

#[cfg(test)]
mod test {
    use std::cell::RefCell;

    use rand::thread_rng;

    use super::*;

    fn storable_roundtrip<T: Storable>(value: &T) -> T {
        T::from_bytes(value.to_bytes())
    }

    #[test]
    fn test_signing_strategy_roundtrip() {
        let signing_strategy = SigningStrategy::Local {
            private_key: [42; 32],
        };
        assert_eq!(storable_roundtrip(&signing_strategy), signing_strategy);
    }

    #[cfg(feature = "ic_sign")]
    #[test]
    fn test_signing_ic_strategy_roundtrip() {
        let signing_strategy = SigningStrategy::ManagementCanister {
            key_id: crate::ic_sign::SigningKeyId::Dfx,
        };
        assert_eq!(storable_roundtrip(&signing_strategy), signing_strategy);
    }

    #[test]
    fn test_local_signer_storable_roundtrip() {
        let wallet = Wallet::new(&mut thread_rng());
        let signer = TxSigner::Local(LocalTxSigner {
            wallet: wallet.clone(),
        });
        let signer: TxSigner = storable_roundtrip(&signer);

        #[allow(irrefutable_let_patterns)]
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

    #[cfg(feature = "ic_sign")]
    #[test]
    fn test_management_canister_signer_roundtrip() {
        let management_canister_signer = ManagementCanisterSigner {
            key_id: crate::ic_sign::SigningKeyId::Dfx,
            cached_address: RefCell::new(Some(H160::from_slice(&[3; 20]))),
            derivation_path: vec![vec![1, 2], vec![3]],
            cached_pubkey: RefCell::new(Some(vec![42; 32])),
        };
        let signer: TxSigner = storable_roundtrip(&TxSigner::ManagementCanister(
            management_canister_signer.clone(),
        ));
        if let TxSigner::ManagementCanister(ManagementCanisterSigner {
            key_id,
            cached_address,
            derivation_path,
            cached_pubkey,
        }) = signer
        {
            assert!(matches!(key_id, crate::ic_sign::SigningKeyId::Dfx));
            assert_eq!(cached_address, management_canister_signer.cached_address);
            assert_eq!(derivation_path, management_canister_signer.derivation_path);
            assert_eq!(cached_pubkey, management_canister_signer.cached_pubkey)
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

        #[allow(irrefutable_let_patterns)]
        if let TxSigner::Local(LocalTxSigner { wallet }) = signer {
            assert_eq!(wallet.chain_id(), 42);
            assert_eq!(wallet.signer().to_bytes().as_slice(), &[2; 32]);
        } else {
            panic!("invalid signer")
        }
    }

    #[cfg(feature = "ic_sign")]
    #[test]
    fn test_create_management_signer() {
        let signing_strategy = SigningStrategy::ManagementCanister {
            key_id: crate::ic_sign::SigningKeyId::Test,
        };
        let chain_id = 42;
        let signer = signing_strategy.make_signer(chain_id).unwrap();
        if let TxSigner::ManagementCanister(ManagementCanisterSigner {
            key_id,
            cached_address,
            derivation_path,
            cached_pubkey,
        }) = signer
        {
            assert_eq!(key_id, crate::ic_sign::SigningKeyId::Test);
            assert_eq!(derivation_path, vec![chain_id.to_be_bytes().to_vec()]);
            assert_eq!(*cached_address.borrow(), None);
            assert_eq!(*cached_pubkey.borrow(), None);
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
