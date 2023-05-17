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

    #[error("cryptocurrency pair already exists")]
    PairExist,

    #[error("cryptocurrency pair doesn't exist")]
    PairNotExist,

    #[error("pair key is too long: {0} > 16")]
    PairKeyTooLong(u64),

    #[error("stable pair not found: {0}")]
    StableError(String),

    #[error("http outcall error: {0}")]
    HttpError(String),
}

impl From<StableError> for Error {
    fn from(err: StableError) -> Self {
        Self::StableError(format!("{err:?}"))
    }
}
