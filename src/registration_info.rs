use candid::{CandidType, Deserialize};

use super::H160;

/// Contains the registration info for a new ic-agent; in particular the minter address and the registration fee
#[derive(Debug, Clone, CandidType, Deserialize)]
pub struct RegistrationInfo {
    pub minter_address: H160,
    pub registration_fee: u64,
}
