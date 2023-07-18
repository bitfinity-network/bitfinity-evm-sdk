use candid::{CandidType, Deserialize};
use did::{Transaction, H160, H256};
use ic_canister::{generate_idl, init, query, update, Canister, Idl, PreUpdate};
use ic_exports::ic_kit::ic;
use ic_exports::Principal;

use crate::error::{Error, Result};
use crate::state::evm::EvmCanister;
use crate::state::{Settings, State};

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
            evm: init_data.evm,
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
    pub fn set_evm_canister_id(&mut self, evm_id: Principal) -> Result<()> {
        self.check_owner(ic::caller())?;
        self.state.config.set_evm(evm_id);
        Ok(())
    }

    #[query]
    pub fn get_account(&self) -> Result<H160> {
        self.state.evm.get_account()
    }

    #[update]
    pub async fn reserve_account(&mut self, principal: Principal, tx_hash: H256) -> Result<()> {
        self.state.evm.reserve_address(principal, tx_hash).await
    }

    #[query]
    pub async fn is_address_reserved(&self, principal: Principal, address: H160) -> Result<bool> {
        self.state.evm.is_address_reserved(address, principal).await
    }

    #[update]
    pub async fn send_raw_transaction(&mut self, tx: Transaction) -> Result<H256> {
        self.check_owner(ic::caller())?;

        self.state.evm.send_raw_transaction(tx).await
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
    pub evm: Principal,
}
