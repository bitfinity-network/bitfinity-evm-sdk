use std::borrow::Cow;
use std::cell::RefCell;

use candid::{CandidType, Deserialize, Principal};
use did::codec::{decode, encode};
use did::{H160, H256};
use ic_stable_structures::{get_memory_by_id, CellStructure, StableCell, Storable};

use super::{EvmCanister, EvmCanisterImpl, MINT_AMOUNT};
use crate::error::{Error, Result};
use crate::memory::{MemoryType, MEMORY_MANAGER};
use crate::state::ACCOUNT_MEMORY_ID;

#[derive(Default, Clone)]
pub struct Account {}

impl Account {
    /// Returns this canister's account in evm if reserved
    pub fn get_account(&self) -> Result<H160> {
        ACCOUNT_DATA_CELL.with(|account_data| {
            if let AccountState::Reserved(address) = account_data.borrow().get() {
                Ok(address.clone())
            } else {
                Err(Error::Internal("Account is not reserved yet".to_string()))
            }
        })
    }

    /// Runs the procedure of reserving this canister's account in evm.
    #[allow(dead_code)]
    pub async fn reserve_account(
        &mut self,
        self_canister_id: Principal,
        address: H160,
        tx_hash: H256,
    ) -> Result<()> {
        // check if account is already reserved or in process
        if ACCOUNT_DATA_CELL.with(|account| {
            if account.borrow().get() == &AccountState::Unreserved {
                account
                    .borrow_mut()
                    .set(AccountState::ReservationInProgress)
                    .expect("failed to update account state");
                false
            } else {
                true
            }
        }) {
            return Err(Error::Internal("Account is already reserved".to_string()));
        }

        let mut evm_impl = EvmCanisterImpl::default();

        // check if the address is reserved
        match evm_impl
            .is_address_reserved(address.clone(), self_canister_id)
            .await
        {
            Err(err) => {
                self.reset();
                return Err(err);
            }
            Ok(is_reserved) => {
                if is_reserved {
                    self.reset();
                    return Err(Error::Internal(format!(
                        "{} is already reserved",
                        address.clone()
                    )));
                }
            }
        }

        // mint EVM native tokens to from address
        if let Err(err) = evm_impl
            .mint_evm_tokens(address.clone(), MINT_AMOUNT.into())
            .await
        {
            self.reset();
            return Err(err);
        }

        // reserve ic agent
        if let Err(err) = evm_impl
            .reserve_address(self_canister_id, tx_hash.clone())
            .await
        {
            self.reset();
            return Err(err);
        }

        ACCOUNT_DATA_CELL.with(|account| {
            account
                .borrow_mut()
                .set(AccountState::Reserved(address))
                .expect("failed to update account state")
        });

        Ok(())
    }

    /// Set the account state as unreserved
    pub fn reset(&mut self) {
        ACCOUNT_DATA_CELL.with(|account| {
            account
                .borrow_mut()
                .set(AccountState::Unreserved)
                .expect("failed to update account state")
        })
    }
}

#[derive(Debug, Default, CandidType, Deserialize, PartialEq, Eq)]
enum AccountState {
    #[default]
    Unreserved,
    ReservationInProgress,
    Reserved(H160),
}

impl Storable for AccountState {
    fn to_bytes(&self) -> Cow<[u8]> {
        encode(self).into()
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        decode(&bytes)
    }
}

thread_local! {
    static ACCOUNT_DATA_CELL: RefCell<StableCell<AccountState, MemoryType>> = {
        RefCell::new(StableCell::new(get_memory_by_id(&MEMORY_MANAGER, ACCOUNT_MEMORY_ID), AccountState::default())
            .expect("stable memory account initialization failed"))
    };
}
