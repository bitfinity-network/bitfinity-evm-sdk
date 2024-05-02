use candid::{CandidType, Nat};
use ic_canister_client::{CanisterClient, CanisterClientResult};
use serde::Deserialize;

use crate::icrc::generic_value::Value;
use crate::icrc1::account::Account;
use crate::icrc1::transfer::{TransferArg, TransferError};
use crate::icrc2::allowance::{Allowance, AllowanceArgs};
use crate::icrc2::approve::{ApproveArgs, ApproveError};
use crate::icrc2::transfer_from::{TransferFromArgs, TransferFromError};

#[derive(CandidType, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct StandardRecord {
    pub name: String,
    pub url: String,
}

/// ICRC-1/ICRC-2 client
#[derive(Debug, Clone)]
pub struct IcrcCanisterClient<C: CanisterClient> {
    client: C,
}

/// Implements the ICRC-1 and ICRC-2 client interfaces.
///
/// The `IcrcCanisterClient` provides a set of methods to interact with an ICRC-1 and ICRC-2 compatible canister.
/// It allows querying metadata, balances, and performing token transfers.
impl<C: CanisterClient> IcrcCanisterClient<C> {
    /// Create a ICRC Client
    ///
    /// # Arguments
    /// * `client` - The canister client.
    pub fn new(client: C) -> Self {
        Self { client }
    }

    // ============================== ICRC-1 ==============================

    pub async fn icrc1_metadata(&self) -> CanisterClientResult<Vec<(String, Value)>> {
        self.client.query("icrc1_metadata", ()).await
    }

    pub async fn icrc1_name(&self) -> CanisterClientResult<String> {
        self.client.query("icrc1_name", ()).await
    }

    pub async fn icrc1_symbol(&self) -> CanisterClientResult<String> {
        self.client.query("icrc1_symbol", ()).await
    }

    pub async fn icrc1_decimals(&self) -> CanisterClientResult<u8> {
        self.client.query("icrc1_decimals", ()).await
    }

    pub async fn icrc1_total_supply(&self) -> CanisterClientResult<Nat> {
        self.client.query("icrc1_total_supply", ()).await
    }

    pub async fn icrc1_fee(&self) -> CanisterClientResult<Nat> {
        self.client.query("icrc1_fee", ()).await
    }

    pub async fn icrc1_supported_standards(&self) -> CanisterClientResult<Vec<StandardRecord>> {
        self.client.query("icrc1_supported_standards", ()).await
    }

    pub async fn icrc1_minting_account(&self) -> CanisterClientResult<Option<Account>> {
        self.client.query("icrc1_minting_account", ()).await
    }

    pub async fn icrc1_balance_of(&self, account: Account) -> CanisterClientResult<Nat> {
        self.client.query("icrc1_balance_of", (account,)).await
    }

    /// Transfers the specified `amount` of tokens from the current subaccount to the
    /// `to` account.
    ///
    /// # Arguments
    ///
    /// - `to`: The account to transfer the tokens to.
    /// - `amount`: The amount of tokens to transfer.
    /// - `from_subaccount`: The optional subaccount to transfer the tokens from.
    ///
    /// # Returns
    ///
    /// A result containing the new balance of the `from_subaccount` after the
    /// transfer, or an error if the transfer failed.
    pub async fn icrc1_transfer(
        &self,
        transfer_args: TransferArg,
    ) -> CanisterClientResult<Result<Nat, TransferError>> {
        self.client.update("icrc1_transfer", (transfer_args,)).await
    }

    // ============================== ICRC-2 ==============================

    /// Returns the current allowance for the specified `owner` and `spender`.
    /// The allowance is the amount of tokens that the `spender` is allowed to
    /// spend on behalf of the `owner`.
    pub async fn icrc2_allowance(&self, args: AllowanceArgs) -> CanisterClientResult<Allowance> {
        self.client.query("icrc2_allowance", (args,)).await
    }

    /// Approves the specified `spender` to spend up to `amount` on behalf of
    /// the `from_subaccount`.
    /// Returns the new allowance amount.
    pub async fn icrc2_approve(
        &self,
        approve: ApproveArgs,
    ) -> CanisterClientResult<Result<Nat, ApproveError>> {
        self.client.update("icrc2_approve", (approve,)).await
    }

    /// Transfers the specified `amount` of tokens from the `from` account
    /// to `to` account.
    pub async fn icrc2_transfer_from(
        &self,
        transfer_args: TransferFromArgs,
    ) -> CanisterClientResult<Result<Nat, TransferFromError>> {
        self.client
            .update("icrc2_transfer_from", (transfer_args,))
            .await
    }
}
