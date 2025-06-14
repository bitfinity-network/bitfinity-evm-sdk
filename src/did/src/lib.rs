//! This crate contains our implementation of some of the `alloy_primitives` type.
//! We have derived `candid::CandidType` for all of the types required, and implemented `From` and `Into` for all for easy conversion between the two.
//! This is required because of `ic` Canisters required all types that are used in `update` and `query` methods to have `candid::CandidType` derived.
//! This module contains submodules for each of the types that we have implemented.

pub mod block;
pub mod build;
pub mod bytes;
pub mod certified;
pub mod codec;
pub mod constant;
pub mod error;
pub mod evm_state;
pub mod gas;
pub mod hash;
pub mod ic;
pub mod init;
pub mod integer;
pub mod keccak;
pub mod logs;
pub mod permission;
pub mod revert_blocks;
pub mod send_raw_transaction;
pub mod state;
pub mod transaction;
pub mod unsafe_blocks;

pub mod fees;
pub mod http;
pub mod rpc;
#[cfg(test)]
mod test_utils;
pub mod upgrade_info;
pub mod utils;

pub use block::Block;
pub use error::{ExitFatal, HaltError};
pub use fees::FeeHistory;
pub use gas::*;
pub use hash::{H64, H160, H256};
pub use integer::{U64, U256};
pub use transaction::{BlockId, BlockNumber, Transaction, TransactionReceipt};

pub use crate::bytes::Bytes;
pub use crate::ic::*;
