//! This crate contains our implementation of some of the `ethers_core::types` type.
//! We have derived `candid::CandidType` for all of the types required, and implemented `From` and `Into` for all for easy conversion between the two.
//! This is required because of `ic` Canisters required all types that are used in `update` and `query` methods to have `candid::CandidType` derived.
//! This module contains submodules for each of the types that we have implemented.

pub mod block;
pub mod build;
pub mod bytes;
pub mod certified_result;
pub mod codec;
pub mod constant;
pub mod error;
pub mod gas;
pub mod hash;
pub mod ic;
pub mod init;
pub mod integer;
pub mod keccak;
pub mod logs;
pub mod mint_order_exemption;
pub mod notify;
pub mod permission;
pub mod state;
pub mod transaction;

pub mod fees;
#[cfg(test)]
mod test_utils;

pub use block::Block;
pub use error::{ExitFatal, HaltError};
pub use fees::FeeHistory;
pub use gas::*;
pub use hash::{H160, H256, H64};
pub use integer::{U256, U64};
pub use mint_order_exemption::MintOrderExemptionUserData;
pub use notify::NotificationInput;
pub use transaction::{BlockId, BlockNumber, Transaction, TransactionReceipt};

pub use crate::bytes::Bytes;
