use candid::CandidType;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, CandidType, Clone, PartialEq, Eq, Debug)]
pub struct CertifiedResult<T> {
    pub data: T,
    pub witness: Vec<u8>,
    pub certificate: Vec<u8>,
}
