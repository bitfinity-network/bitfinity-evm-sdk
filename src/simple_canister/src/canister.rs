use candid::{CandidType, Deserialize};
use ic_canister::{generate_idl, init, query, update, Canister, Idl, PreUpdate};
use ic_exports::ic_kit::ic;
use ic_exports::Principal;

use crate::error::{Error, Result};
use crate::state::{
    evm::did::{Transaction, H160, H256, U256},
    evm::EvmCanister,
    Settings, State,
};

/// A canister to transfer funds between IC token canisters and EVM canister contracts.
#[derive(Canister)]
pub struct TempCanister {
    #[id]
    id: Principal,

    state: State,
}

impl PreUpdate for TempCanister {}

impl TempCanister {
    /// Initialize the canister with given data.
    #[init]
    pub fn init(&mut self, init_data: InitData) {
        let settings = Settings {
            owner: init_data.owner,
            evmc: init_data.evmc,
        };

        self.state.reset(settings);
    }

    /// Returns principal of canister owner.
    #[query]
    pub fn get_owner(&self) -> Principal {
        self.state.config.get_owner()
    }

    /// Sets a new principal for canister owner.
    ///
    /// This method should be called only by current owner,
    /// else `Error::NotAuthorized` will be returned.
    #[update]
    pub fn set_owner(&mut self, owner: Principal) -> Result<()> {
        self.check_owner(ic::caller())?;
        self.state.config.set_owner(owner);
        Ok(())
    }

    /// Returns principal of evm canister id.
    #[query]
    pub fn get_evm_canister_id(&self) -> Principal {
        self.state.config.get_evm_canister_id()
    }

    /// Sets a new principal for evm canister id.
    ///
    /// This method should be called only by current owner,
    /// else `Error::NotAuthorized` will be returned.
    #[update]
    pub fn set_evm_canister_id(&mut self, evmc_id: Principal) -> Result<()> {
        self.check_owner(ic::caller())?;
        self.state.config.set_evmc(evmc_id);
        Ok(())
    }

    #[query]
    pub fn get_account(&self) -> Result<H160> {
        self.state.evm.get_account()
    }

    #[update]
    pub async fn register_account(
        &mut self,
        transaction: Transaction,
        signing_key: Vec<u8>,
    ) -> Result<()> {
        self.check_owner(ic::caller())?;
        let canister_id = ic::id();

        self.state
            .evm
            .register_account(transaction, signing_key, canister_id)
            .await
    }

    #[update]
    pub async fn transact(&mut self, value: U256, to: H160, data: Vec<u8>) -> Result<H256> {
        self.check_owner(ic::caller())?;

        self.state.evm.transact(value, to, data, None).await
    }

    #[update]
    pub async fn create_contract(
        &mut self,
        value: U256,
        code: Vec<u8>,
        gas_limit: u64,
    ) -> Result<H256> {
        self.check_owner(ic::caller())?;

        self.state
            .evm
            .create_contract(value, code, Some(gas_limit))
            .await
    }

    fn check_owner(&self, principal: Principal) -> Result<()> {
        let owner = self.state.config.get_owner();
        if owner == principal || owner == Principal::anonymous() {
            return Ok(());
        }
        Err(Error::NotAuthorized)
    }

    /// Returns candid IDL.
    /// This should be the last fn to see previous endpoints in macro.
    pub fn idl() -> Idl {
        generate_idl!()
    }
}

/// Minter canister initialization data.
#[derive(Debug, Deserialize, CandidType, Clone, Copy)]
pub struct InitData {
    /// Principal of canister's owner.
    pub owner: Principal,
    /// Principal of evm canister id.
    pub evmc: Principal,
}
