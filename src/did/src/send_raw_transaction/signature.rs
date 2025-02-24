use candid::CandidType;
use serde::{Deserialize, Serialize};

use crate::{U256, U64};

/// A signature is a pair of integers (r, s) that are used to sign transactions.
/// The signature also contains a recovery id v that is used to recover the public key from the signature.
/// The public key is then used to verify the signature.
#[derive(Debug, Clone, PartialEq, Eq, CandidType, Serialize, Deserialize)]
pub struct Signature {
    pub v: U64,
    pub r: U256,
    pub s: U256,
}
