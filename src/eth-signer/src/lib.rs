use alloy::signers::local::PrivateKeySigner;

#[cfg(feature = "ic_sign")]
pub mod ic_sign;
pub mod sign_strategy;
pub mod transaction;

/// A wallet instantiated with a locally stored private key
pub type LocalWallet = PrivateKeySigner;
pub type SignerError = alloy::signers::Error;
