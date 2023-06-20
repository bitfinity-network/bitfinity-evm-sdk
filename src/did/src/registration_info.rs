use candid::{CandidType, Deserialize, Principal};

use super::H160;

/// Contains the registration info for a new ic-agent; in particular the minter address and the registration fee
#[derive(Debug, Clone, CandidType, Deserialize)]
pub struct RegistrationInfo {
    pub minter_address: H160,
    pub registration_fee: u64,
}

/// EVM address registration status
#[derive(Debug, Clone, CandidType, Deserialize, PartialEq, Eq)]
pub enum AddressRegistrationStatus {
    /// No principal registered for this address
    Unregistered,
    /// Address is registered but not verified
    Registered,
    /// Address is verified for the given principal
    Verified(Principal),
}
