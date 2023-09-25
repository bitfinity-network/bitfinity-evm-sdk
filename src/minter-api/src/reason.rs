use candid::{CandidType, Principal};
use did::{H160, U256};
use ic_exports::icrc_types::icrc1::account::Subaccount;
use serde::Deserialize;

/// Information to perform burn operation for ICRC-2 token and create a mint order.
#[derive(Debug, Clone, Deserialize, CandidType)]
pub struct Icrc2Burn {
    /// Amount to burn;
    pub amount: U256,

    /// Principal of ICRC-2 token to burn.
    pub icrc2_token_principal: Principal,

    /// Subaccount of the ICRC-2 token from which amount will be burned.
    pub from_subaccount: Option<Subaccount>,

    /// Address of the Wrapped token recipient.
    pub recipient_address: H160,

    /// This ID will be a key for stored MintOrder related with this ICRC-2 burn.
    pub operation_id: u32,
}

