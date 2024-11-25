use std::error::Error;

use alloy::{consensus::{SignableTransaction, Signed, TxEnvelope, TxLegacy}, network::{TransactionBuilder, TxSignerSync}, primitives::{Address, U256}, rpc::types::TransactionRequest, signers::{k256::ecdsa::SigningKey, local::PrivateKeySigner, utils::secret_key_to_address}};
use async_trait::async_trait;

#[cfg(feature = "ic_sign")]
pub mod ic_sign;
pub mod sign_strategy;
pub mod transaction;

// /// A wallet instantiated with a locally stored private key
// // pub type LocalWallet<'a> = Wallet<'a, ethers_core::k256::ecdsa::SigningKey>;
pub type LocalWallet = PrivateKeySigner;
#[deprecated]
pub type WalletError = alloy::signers::Error;
pub type SignerError = alloy::signers::Error;

// /// Applies [EIP155](https://github.com/ethereum/EIPs/blob/master/EIPS/eip-155.md)
// pub fn to_eip155_v<T: Into<u8>>(recovery_id: T, chain_id: u64) -> u64 {
//     (recovery_id.into() as u64) + 35 + chain_id * 2
// }

// / Trait for signing transactions and messages
// /
// / Implement this trait to support different signing modes, e.g. Ledger, hosted etc.
// #[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
// #[cfg_attr(not(target_arch = "wasm32"), async_trait)]
// pub trait Signer: std::fmt::Debug + Send + Sync {
//     type Error: Error + Send + Sync;

//     /// Signs the hash of the provided message after prefixing it
//     async fn sign_message<S: Send + Sync + AsRef<[u8]>>(
//         &self,
//         message: S,
//     ) -> Result<Signature, Self::Error>;

//     /// Signs the transaction
//     async fn sign_transaction(&self, message: &TypedTransaction) -> Result<Signature, Self::Error>;

//     /// Returns the signer's Ethereum Address
//     fn address(&self) -> Address;

//     /// Returns the signer's chain id
//     fn chain_id(&self) -> u64;

//     /// Sets the signer's chain id
//     #[must_use]
//     fn with_chain_id<T: Into<u64>>(self, chain_id: T) -> Self;
// }

#[test]
fn signed_tx_json() {

    let chain_id = 1;
    let key = SigningKey::random(&mut rand::thread_rng());
    let from = secret_key_to_address(&key);
    let signer = PrivateKeySigner::new_with_credential(key, from, Some(chain_id));

    let tx = TransactionRequest::default()
    .with_from(from)
    .with_to(Address::ZERO)
    .with_value(U256::from(10u64))
    .with_chain_id(chain_id)
    .with_nonce(0)
    .with_gas_price(10)
    .with_gas_limit(53000)
    .build_consensus_tx().unwrap();

    let mut tx = tx.legacy().cloned().unwrap();
    let signature = signer.sign_transaction_sync(&mut tx).unwrap();

    let signed: Signed<TxLegacy> = tx.into_signed(signature);
    
    // SignedTx Json roundtrip -> OK
    {
        let signed_json = serde_json::to_string(&signed).unwrap();
        let signed_tx: Signed<TxLegacy> = serde_json::from_str(&signed_json).unwrap();
    }

    // TxEnvelop Json roundtrip -> OK
    {
        let tx_envelop: TxEnvelope = signed.clone().into();
        let tx_envelop_json = serde_json::to_string(&tx_envelop).unwrap();
        let decoded_tx_envelop: TxEnvelope = serde_json::from_str(&tx_envelop_json).unwrap();
    }

    {
        let rpc_tx = alloy::rpc::types::Transaction::<TxEnvelope> {
            inner: signed.into(),
            from,
            block_hash: None,
            block_number: None,
            transaction_index: None,
            effective_gas_price: None,
        };

        let rpc_tx_json = serde_json::to_string(&rpc_tx).unwrap();
        println!("{}", rpc_tx_json);
        let decoded_rpc_tx: alloy::rpc::types::Transaction::<TxEnvelope> = serde_json::from_str(&rpc_tx_json).unwrap();
    }
    


}