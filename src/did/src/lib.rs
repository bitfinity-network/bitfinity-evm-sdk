//! This crate contains our implementation of some of the `ethers_core::types` type.
//! We have derived `candid::CandidType` for all of the types required, and implemented `From` and `Into` for all for easy conversion between the two.
//! This is required because of `ic` Canisters required all types that are used in `update` and `query` methods to have `candid::CandidType` derived.
//! This module contains submodules for each of the types that we have implemented.

pub mod block;
pub mod bytes;
pub mod codec;
pub mod constant;
pub mod error;
pub mod hash;
pub mod integer;
pub mod keccak;
pub mod notify;
#[cfg(feature = "signer")]
pub mod sign_strategy;
pub mod transaction;

#[cfg(test)]
mod test_utils;

pub use block::Block;
use candid::{CandidType, Deserialize};
pub use error::{ExitFatal, HaltError};
pub use hash::{H160, H256, H64};
pub use integer::{U256, U64};
pub use transaction::{BlockNumber, Transaction, TransactionReceipt};
pub use notify::NotificaionTx;

pub use crate::bytes::Bytes;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, CandidType)]
pub struct BasicAccount {
    /// Account balance.
    pub balance: U256,
    /// Account nonce.
    pub nonce: U256,
}
