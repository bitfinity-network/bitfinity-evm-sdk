use candid::Principal;
use did::build::BuildData;
use did::H160;
use ic_canister_client::{CanisterClient, CanisterClientResult};
use minter_did::error::Result as McResult;
use minter_did::id256::Id256;
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

    /// Creates ERC-20 mint order for ICRC-2 tokens burning and sends it to the BFTBridge.
    /// Returns operation id.
    pub async fn burn_icrc2(&self, reason: Icrc2Burn) -> CanisterClientResult<McResult<u32>> {
        self.client.update("burn_icrc2", (reason,)).await
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

    /// Returns mint order for the given parameters.
    pub async fn get_mint_order(
        &self,
        sender: Id256,
        src_token: Id256,
        operation_id: u32,
    ) -> CanisterClientResult<Option<SignedMintOrder>> {
        self.client
            .query("get_mint_order", (sender, src_token, operation_id))
            .await
    }

    /// Returns the build data of the canister.
    pub async fn get_canister_build_data(&self) -> CanisterClientResult<BuildData> {
        self.client.query("get_canister_build_data", ()).await
    }
}
