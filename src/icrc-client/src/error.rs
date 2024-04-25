use candid::{CandidType, Deserialize};
use ic_exports::icrc_types::icrc1::transfer::TransferError;
use ic_exports::icrc_types::icrc2::approve::ApproveError;
use thiserror::Error;

pub type IcrcResult<T> = Result<T, IcrcError>;

#[derive(Error, Debug, Deserialize, CandidType)]
pub enum IcrcError {
    #[error("transfer error {0}")]
    Transfer(TransferError),
    #[error("ICRC-2 approve failed: {0:?}")]
    Approve(ApproveError),
}
