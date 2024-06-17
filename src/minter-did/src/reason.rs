use candid::{CandidType, Principal};
use did::{H160, U256};
use ic_exports::icrc_types::icrc1::account::Subaccount;
use serde::{Deserialize, Serialize};

/// Information to perform burn operation for ICRC-2 token and create a mint order.
#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct Icrc2Burn {
    /// Principal from which tokens should be sent.
    pub sender: Principal,

    /// Amount to burn;
    pub amount: U256,

    /// Principal of ICRC-2 token to burn.
    pub icrc2_token_principal: Principal,

    /// Subaccount of the ICRC-2 token from which amount will be burned.
    pub from_subaccount: Option<Subaccount>,

    /// Address of the Wrapped token recipient.
    pub recipient_address: H160,

    /// If user want's mint operation to approve minted tokens,
    /// he can use this field.
    pub approve_after_mint: Option<ApproveAfterMint>,

    /// Address from which fee should be charged for mint transaction
    /// performed by minter canister.
    /// If None, mint transaction will not be sent and user can send it by himself.
    pub fee_payer: Option<H160>,
}

#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct ApproveAfterMint {
    /// Approve minted tokens using this address as a spender.
    pub approve_spender: H160,

    /// Amount to approve.
    pub approve_amount: U256,
}
