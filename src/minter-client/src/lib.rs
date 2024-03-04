use candid::{Nat, Principal};
use did::build::BuildData;
use did::H160;
use ic_canister_client::{CanisterClient, CanisterClientResult};
use minter_did::error::Result as McResult;
use minter_did::id256::Id256;
use minter_did::init::OperationPricing;
use minter_did::order::SignedMintOrder;
use minter_did::reason::Icrc2Burn;

pub struct MinterCanisterClient<C> {
    client: C,
}

impl<C: CanisterClient> MinterCanisterClient<C> {
    pub fn new(client: C) -> Self {
        Self { client }
    }

    /// Updates the runtime configuration of the logger with a new filter in the same form as the `RUST_LOG`
    /// environment variable.
    ///
    /// Example of valid filters:
    /// - info
    /// - debug,crate1::mod1=error,crate1::mod2,crate2=debug
    ///
    /// This method is only for canister owner.
    pub async fn set_logger_filter(&self, filter: String) -> CanisterClientResult<McResult<()>> {
        self.client.update("set_logger_filter", (filter,)).await
    }

    /// Gets the logs
    ///
    /// # Arguments
    /// - `count` is the number of logs to return
    ///
    /// This method is only for canister owner.
    pub async fn ic_logs(&self, count: usize) -> CanisterClientResult<McResult<Vec<String>>> {
        self.client.update("ic_logs", (count,)).await
    }

    /// Returns principal of canister owner.
    pub async fn get_owner(&self) -> CanisterClientResult<Principal> {
        self.client.query("get_owner", ()).await
    }

    /// Sets a new principal for canister owner.
    ///
    /// This method should be called only by current owner,
    /// else `Error::NotAuthorised` will be returned.
    pub async fn set_owner(&mut self, owner: Principal) -> CanisterClientResult<McResult<()>> {
        self.client.update("set_owner", (owner,)).await
    }

    /// Returns principal of EVM canister with which the minter canister works.
    pub async fn get_evm_principal(&self) -> CanisterClientResult<Principal> {
        self.client.query("get_evm_principal", ()).await
    }

    /// Sets principal of EVM canister with which the minter canister works.
    ///
    /// This method should be called only by current owner,
    /// else `Error::NotAuthorised` will be returned.
    pub async fn set_evm_principal(
        &mut self,
        evm: Principal,
    ) -> CanisterClientResult<McResult<()>> {
        self.client.update("set_evm_principal", (evm,)).await
    }

    /// Returns the address of the BFT bridge contract in EVM canister.
    pub async fn get_bft_bridge_contract(&self) -> CanisterClientResult<McResult<Option<H160>>> {
        self.client.update("get_bft_bridge_contract", ()).await
    }

    /// Registers BftBridge contract for EVM canister.
    /// This method is available for canister owner only.
    pub async fn register_evmc_bft_bridge(
        &self,
        bft_bridge_address: H160,
    ) -> CanisterClientResult<McResult<()>> {
        self.client
            .update("register_evmc_bft_bridge", (bft_bridge_address,))
            .await
    }

    /// Returns operation points number of the user.
    pub async fn get_user_operation_points(
        &self,
        user: Option<Principal>,
    ) -> CanisterClientResult<u32> {
        self.client
            .query("get_user_operation_points", (user,))
            .await
    }

    /// Returns operations pricing.
    /// This method is available for canister owner only.
    pub async fn set_operation_pricing(
        &mut self,
        pricing: OperationPricing,
    ) -> CanisterClientResult<McResult<()>> {
        self.client
            .update("set_operation_pricing", (pricing,))
            .await
    }

    /// Returns operation pricing.
    pub async fn get_operation_pricing(&self) -> CanisterClientResult<OperationPricing> {
        self.client.query("get_operation_pricing", ()).await
    }

    /// Creates ERC-20 mint order for ICRC-2 tokens burning.
    pub async fn create_erc_20_mint_order(
        &self,
        reason: Icrc2Burn,
    ) -> CanisterClientResult<McResult<SignedMintOrder>> {
        self.client
            .update("create_erc_20_mint_order", (reason,))
            .await
    }

    /// Returns `(nonce, mint_order)` pairs for the given sender id.
    pub async fn list_mint_orders(
        &self,
        sender: Id256,
        src_token: Id256,
    ) -> CanisterClientResult<Vec<(u32, SignedMintOrder)>> {
        self.client
            .query("list_mint_orders", (sender, src_token))
            .await
    }

    /// Approves ICRC-2 token transfer from minter canister to recipient.
    /// Returns approved amount.
    ///
    /// # Arguments
    /// - `user` is an address of wallet which has been used for Wrapped token burning.
    /// - `operation_id` is an ID retuned by `BFTBridge::burn()` operation.
    pub async fn start_icrc2_mint(
        &self,
        user: &H160,
        operation_id: u32,
    ) -> CanisterClientResult<McResult<Nat>> {
        self.client
            .update("start_icrc2_mint", (user, operation_id))
            .await
    }

    /// Transfers ICRC-2 tokens from minter canister to recipient.
    ///
    /// Before it can be used, ICRC-2 token must be approved by `start_icrc2_mint` which approves the transfer.
    /// After the approval, user should finalize Wrapped token burning, using `BFTBridge::finish_burn()`.
    pub async fn finish_icrc2_mint(
        &self,
        operation_id: u32,
        address: &H160,
        icrc2_token: Principal,
        recipient: Principal,
        amount: Nat,
    ) -> CanisterClientResult<McResult<Nat>> {
        self.client
            .update(
                "finish_icrc2_mint",
                (operation_id, address, icrc2_token, recipient, amount),
            )
            .await
    }

    /// Returns the build data of the canister.
    pub async fn get_canister_build_data(&self) -> CanisterClientResult<BuildData> {
        self.client.query("get_canister_build_data", ()).await
    }
}
