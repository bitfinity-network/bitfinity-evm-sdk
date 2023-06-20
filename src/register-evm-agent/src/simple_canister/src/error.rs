use candid::{CandidType, Deserialize};
use ic_stable_structures::Error as StableError;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error, Deserialize, CandidType, Eq, PartialEq)]
pub enum Error {
    #[error("internal error: {0}")]
    Internal(String),

    #[error("the user has no permission to call this method")]
    NotAuthorized,

    #[error("stable pair not found: {0}")]
    StableError(String),
}

impl From<StableError> for Error {
    fn from(err: StableError) -> Self {
        Self::StableError(format!("{err:?}"))
    }
}
