//! Specific helper functions for loading an offline K256 Private Key stored on disk
use std::borrow::Cow;
use std::str::FromStr;

use ethers_core::k256::ecdsa::{self, SigningKey};
use ethers_core::rand::{CryptoRng, Rng};
use ethers_core::utils;
use thiserror::Error;

use super::Wallet;

#[derive(Error, Debug)]
/// Error thrown by the Wallet module
pub enum WalletError {
    /// Error propagated from k256's ECDSA module
    #[error(transparent)]
    EcdsaError(#[from] ecdsa::Error),
    /// Error propagated from the hex crate.
    #[error(transparent)]
    HexError(#[from] hex::FromHexError),
    /// Error propagated by IO operations
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    /// Error type from Eip712Error message
    #[error("error encoding eip712 struct: {0:?}")]
    Eip712Error(String),
}

impl<'a> Wallet<'a, SigningKey> {
    /// Creates a new random keypair seeded with the provided RNG
    pub fn new<R: Rng + CryptoRng>(rng: &mut R) -> Self {
        let signer = SigningKey::random(rng);
        let address = utils::secret_key_to_address(&signer);
        Self {
            signer: Cow::Owned(signer),
            address,
            chain_id: 1,
        }
    }

    /// Creates a new Wallet instance from a raw scalar value (big endian).
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, WalletError> {
        let signer = SigningKey::from_slice(bytes)?;
        let address = utils::secret_key_to_address(&signer);
        Ok(Self {
            signer: Cow::Owned(signer),
            address,
            chain_id: 1,
        })
    }
}

impl<'a> PartialEq for Wallet<'a, SigningKey> {
    fn eq(&self, other: &Self) -> bool {
        self.signer.to_bytes().eq(&other.signer.to_bytes())
            && self.address == other.address
            && self.chain_id == other.chain_id
    }
}

impl From<SigningKey> for Wallet<'static, SigningKey> {
    fn from(signer: SigningKey) -> Self {
        let address = utils::secret_key_to_address(&signer);

        Self {
            signer: Cow::Owned(signer),
            address,
            chain_id: 1,
        }
    }
}

use ethers_core::k256::SecretKey as K256SecretKey;

impl From<K256SecretKey> for Wallet<'static, SigningKey> {
    fn from(key: K256SecretKey) -> Self {
        let signer = key.into();
        let address = utils::secret_key_to_address(&signer);

        Self {
            signer: Cow::Owned(signer),
            address,
            chain_id: 1,
        }
    }
}

impl FromStr for Wallet<'static, SigningKey> {
    type Err = WalletError;

    fn from_str(src: &str) -> Result<Self, Self::Err> {
        let src = src
            .strip_prefix("0x")
            .or_else(|| src.strip_prefix("0X"))
            .unwrap_or(src);
        let src = hex::decode(src)?;
        let sk = SigningKey::from_bytes(src.as_slice().into())?;
        Ok(sk.into())
    }
}

impl TryFrom<&str> for Wallet<'static, SigningKey> {
    type Error = WalletError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl TryFrom<String> for Wallet<'static, SigningKey> {
    type Error = WalletError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

#[cfg(test)]
mod tests {
    use ethers_core::types::Address;

    use super::*;
    use crate::Signer;

    #[tokio::test]
    async fn signs_msg() {
        let message = "Some data";
        let hash = ethers_core::utils::hash_message(message);
        let key = Wallet::<SigningKey>::new(&mut rand::thread_rng());
        let address = key.address;

        // sign a message
        let signature = key.sign_message(message).await.unwrap();

        // ecrecover via the message will hash internally
        let recovered = signature.recover(message).unwrap();

        // if provided with a hash, it will skip hashing
        let recovered2 = signature.recover(hash).unwrap();

        // verifies the signature is produced by `address`
        signature.verify(message, address).unwrap();

        assert_eq!(recovered, address);
        assert_eq!(recovered2, address);
    }

    #[tokio::test]
    async fn signs_tx() {
        use ethers_core::types::{TransactionRequest, U64};

        use crate::TypedTransaction;
        // retrieved test vector from:
        // https://web3js.readthedocs.io/en/v1.2.0/web3-eth-accounts.html#eth-accounts-signtransaction
        let tx: TypedTransaction = TransactionRequest {
            from: None,
            to: Some(
                "F0109fC8DF283027b6285cc889F5aA624EaC1F55"
                    .parse::<Address>()
                    .unwrap()
                    .into(),
            ),
            value: Some(1_000_000_000.into()),
            gas: Some(2_000_000.into()),
            nonce: Some(0.into()),
            gas_price: Some(21_000_000_000u128.into()),
            data: None,
            chain_id: Some(U64::one()),
        }
        .into();
        let wallet: Wallet<SigningKey> =
            "4c0883a69102937d6231471b5dbb6204fe5129617082792ae468d01a3f362318"
                .parse()
                .unwrap();
        let wallet = wallet.with_chain_id(tx.chain_id().unwrap().as_u64());

        let sig = wallet.sign_transaction(&tx).await.unwrap();
        let sighash = tx.sighash();
        sig.verify(sighash, wallet.address).unwrap();
    }

    #[tokio::test]
    async fn signs_tx_empty_chain_id() {
        use ethers_core::types::TransactionRequest;

        use crate::TypedTransaction;
        // retrieved test vector from:
        // https://web3js.readthedocs.io/en/v1.2.0/web3-eth-accounts.html#eth-accounts-signtransaction
        let tx: TypedTransaction = TransactionRequest {
            from: None,
            to: Some(
                "F0109fC8DF283027b6285cc889F5aA624EaC1F55"
                    .parse::<Address>()
                    .unwrap()
                    .into(),
            ),
            value: Some(1_000_000_000.into()),
            gas: Some(2_000_000.into()),
            nonce: Some(0.into()),
            gas_price: Some(21_000_000_000u128.into()),
            data: None,
            chain_id: None,
        }
        .into();
        let wallet: Wallet<SigningKey> =
            "4c0883a69102937d6231471b5dbb6204fe5129617082792ae468d01a3f362318"
                .parse()
                .unwrap();
        let wallet = wallet.with_chain_id(1u64);

        // this should populate the tx chain_id as the signer's chain_id (1) before signing
        let sig = wallet.sign_transaction(&tx).await.unwrap();

        // since we initialize with None we need to re-set the chain_id for the sighash to be
        // correct
        let mut tx = tx;
        tx.set_chain_id(1);
        let sighash = tx.sighash();
        sig.verify(sighash, wallet.address).unwrap();
    }

    #[test]
    fn signs_tx_empty_chain_id_sync() {
        use ethers_core::types::TransactionRequest;

        use crate::TypedTransaction;

        let chain_id = 1337u64;
        // retrieved test vector from:
        // https://web3js.readthedocs.io/en/v1.2.0/web3-eth-accounts.html#eth-accounts-signtransaction
        let tx: TypedTransaction = TransactionRequest {
            from: None,
            to: Some(
                "F0109fC8DF283027b6285cc889F5aA624EaC1F55"
                    .parse::<Address>()
                    .unwrap()
                    .into(),
            ),
            value: Some(1_000_000_000u64.into()),
            gas: Some(2_000_000u64.into()),
            nonce: Some(0u64.into()),
            gas_price: Some(21_000_000_000u128.into()),
            data: None,
            chain_id: None,
        }
        .into();
        let wallet: Wallet<SigningKey> =
            "4c0883a69102937d6231471b5dbb6204fe5129617082792ae468d01a3f362318"
                .parse()
                .unwrap();
        let wallet = wallet.with_chain_id(chain_id);

        // this should populate the tx chain_id as the signer's chain_id (1337) before signing and
        // normalize the v
        let sig = wallet.sign_transaction_sync(&tx).unwrap();

        // ensure correct v given the chain - first extract recid
        let recid = (sig.v - 35) % 2;
        // eip155 check
        assert_eq!(sig.v, chain_id * 2 + 35 + recid);

        // since we initialize with None we need to re-set the chain_id for the sighash to be
        // correct
        let mut tx = tx;
        tx.set_chain_id(chain_id);
        let sighash = tx.sighash();
        sig.verify(sighash, wallet.address).unwrap();
    }

    #[test]
    fn key_to_address() {
        let wallet: Wallet<SigningKey> =
            "0000000000000000000000000000000000000000000000000000000000000001"
                .parse()
                .unwrap();
        assert_eq!(
            wallet.address,
            Address::from_str("7E5F4552091A69125d5DfCb7b8C2659029395Bdf").expect("Decoding failed")
        );

        let wallet: Wallet<SigningKey> =
            "0000000000000000000000000000000000000000000000000000000000000002"
                .parse()
                .unwrap();
        assert_eq!(
            wallet.address,
            Address::from_str("2B5AD5c4795c026514f8317c7a215E218DcCD6cF").expect("Decoding failed")
        );

        let wallet: Wallet<SigningKey> =
            "0000000000000000000000000000000000000000000000000000000000000003"
                .parse()
                .unwrap();
        assert_eq!(
            wallet.address,
            Address::from_str("6813Eb9362372EEF6200f3b1dbC3f819671cBA69").expect("Decoding failed")
        );
    }

    #[test]
    fn key_from_bytes() {
        let wallet: Wallet<SigningKey> =
            "0000000000000000000000000000000000000000000000000000000000000001"
                .parse()
                .unwrap();

        let key_as_bytes = wallet.signer.to_bytes();
        let wallet_from_bytes = Wallet::from_bytes(&key_as_bytes).unwrap();

        assert_eq!(wallet.address, wallet_from_bytes.address);
        assert_eq!(wallet.chain_id, wallet_from_bytes.chain_id);
        assert_eq!(wallet.signer, wallet_from_bytes.signer);
    }

    #[test]
    fn key_from_str() {
        let wallet: Wallet<SigningKey> =
            "0000000000000000000000000000000000000000000000000000000000000001"
                .parse()
                .unwrap();

        // Check FromStr and `0x`
        let wallet_0x: Wallet<SigningKey> =
            "0x0000000000000000000000000000000000000000000000000000000000000001"
                .parse()
                .unwrap();
        assert_eq!(wallet.address, wallet_0x.address);
        assert_eq!(wallet.chain_id, wallet_0x.chain_id);
        assert_eq!(wallet.signer, wallet_0x.signer);

        // Check FromStr and `0X`
        let wallet_0x_cap: Wallet<SigningKey> =
            "0X0000000000000000000000000000000000000000000000000000000000000001"
                .parse()
                .unwrap();
        assert_eq!(wallet.address, wallet_0x_cap.address);
        assert_eq!(wallet.chain_id, wallet_0x_cap.chain_id);
        assert_eq!(wallet.signer, wallet_0x_cap.signer);

        // Check TryFrom<&str>
        let wallet_0x_tryfrom_str: Wallet<SigningKey> =
            "0x0000000000000000000000000000000000000000000000000000000000000001"
                .try_into()
                .unwrap();
        assert_eq!(wallet.address, wallet_0x_tryfrom_str.address);
        assert_eq!(wallet.chain_id, wallet_0x_tryfrom_str.chain_id);
        assert_eq!(wallet.signer, wallet_0x_tryfrom_str.signer);

        // Check TryFrom<String>
        let wallet_0x_tryfrom_string: Wallet<SigningKey> =
            "0x0000000000000000000000000000000000000000000000000000000000000001"
                .to_string()
                .try_into()
                .unwrap();
        assert_eq!(wallet.address, wallet_0x_tryfrom_string.address);
        assert_eq!(wallet.chain_id, wallet_0x_tryfrom_string.chain_id);
        assert_eq!(wallet.signer, wallet_0x_tryfrom_string.signer);

        // Must fail because of `0z`
        "0z0000000000000000000000000000000000000000000000000000000000000001"
            .parse::<Wallet<SigningKey>>()
            .unwrap_err();
    }
}
