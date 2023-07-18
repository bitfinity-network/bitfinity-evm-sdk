use std::cell::RefCell;

use account::Account;
use async_trait::async_trait;
use candid::Principal;
// use
use did::{
    error::{EvmError, TransactionPoolError},
    BasicAccount, Transaction, TransactionReceipt, H160, H256, U256,
};
use ic_exports::ic_kit::{ic, RejectionCode};
use ic_stable_structures::StableCell;

use crate::error::Error;
use crate::state::{State, NONCE_MEMORY_ID};

mod account;

// Registry agent fee + other transfer example
pub const MINT_AMOUNT: u64 = 10_000_000;

type EvmResult<T> = Result<T, EvmError>;

#[derive(Default)]
pub struct EvmCanisterImpl {}

impl EvmCanisterImpl {
    fn get_evm_canister_id(&self) -> Principal {
        State::default().config.get_evm_canister_id()
    }

    fn process_call<T>(
        &self,
        result: Result<T, (RejectionCode, std::string::String)>,
    ) -> Result<T, Error> {
        result.map_err(|e| Error::Internal(format!("ic call failure: {e:?}")))
    }

    fn process_call_result<T>(
        &self,
        result: Result<EvmResult<T>, (RejectionCode, std::string::String)>,
    ) -> Result<T, Error> {
        let result = self.process_call(result)?;
        if let Err(EvmError::TransactionPool(TransactionPoolError::InvalidNonce {
            expected, ..
        })) = &result
        {
            NONCE_CELL.with(|nonce| {
                nonce
                    .borrow_mut()
                    .set(expected.clone())
                    .expect("failed to update nonce");
            });
        }

        result.map_err(|e| Error::Internal(format!("transaction error: {e}")))
    }

    pub fn get_account(&self) -> Result<H160, Error> {
        Account::default().get_account()
    }
}

/// Interface for calling EVMC methods

#[async_trait(?Send)]
pub trait EvmCanister: Send {
    async fn get_balance(&self, address: H160) -> Result<U256, Error>;

    async fn get_transaction_by_hash(&self, tx_hash: H256) -> Result<Option<Transaction>, Error>;

    async fn get_transaction_receipt_by_hash(
        &self,
        tx_hash: H256,
    ) -> Result<Option<TransactionReceipt>, Error>;

    async fn mint_evm_tokens(&mut self, to: H160, amount: U256) -> Result<U256, Error>;

    async fn reserve_address(&mut self, principal: Principal, tx_hash: H256) -> Result<(), Error>;

    async fn is_address_reserved(&self, address: H160, principal: Principal)
        -> Result<bool, Error>;

    async fn send_raw_transaction(&self, tx: Transaction) -> Result<H256, Error>;
}

#[async_trait(?Send)]
impl EvmCanister for EvmCanisterImpl {
    async fn get_balance(&self, address: H160) -> Result<U256, Error> {
        let res: Result<(BasicAccount,), _> =
            ic::call(self.get_evm_canister_id(), "account_basic", (address,)).await;

        self.process_call(res.map(|val| val.0))
            .map(|acc| acc.balance)
    }

    async fn get_transaction_by_hash(&self, tx_hash: H256) -> Result<Option<Transaction>, Error> {
        let res: Result<(Option<Transaction>,), _> = ic::call(
            self.get_evm_canister_id(),
            "eth_get_transaction_by_hash",
            (tx_hash,),
        )
        .await;

        self.process_call(res.map(|val| val.0))
    }

    async fn get_transaction_receipt_by_hash(
        &self,
        tx_hash: H256,
    ) -> Result<Option<TransactionReceipt>, Error> {
        let res: Result<(Option<TransactionReceipt>,), _> = ic::call(
            self.get_evm_canister_id(),
            "eth_get_transaction_receipt",
            (tx_hash,),
        )
        .await;

        self.process_call(res.map(|val| val.0))
    }

    async fn mint_evm_tokens(&mut self, to: H160, amount: U256) -> Result<U256, Error> {
        let res: Result<(EvmResult<U256>,), _> =
            ic::call(self.get_evm_canister_id(), "mint_evm_tokens", (to, amount)).await;

        self.process_call_result(res.map(|val| val.0))
    }

    async fn reserve_address(&mut self, principal: Principal, tx_hash: H256) -> Result<(), Error> {
        let res: Result<(EvmResult<()>,), _> = ic::call(
            self.get_evm_canister_id(),
            "reserve_address",
            (principal, tx_hash),
        )
        .await;

        self.process_call_result(res.map(|val| val.0))
    }

    async fn is_address_reserved(
        &self,
        address: H160,
        principal: Principal,
    ) -> Result<bool, Error> {
        let res: Result<(bool,), _> = ic::call(
            self.get_evm_canister_id(),
            "is_address_reserved",
            (address, principal),
        )
        .await;

        self.process_call(res.map(|val| val.0))
    }

    async fn send_raw_transaction(&self, tx: Transaction) -> Result<H256, Error> {
        let res: Result<(EvmResult<H256>,), _> = ic::call(
            self.get_evm_canister_id(),
            "eth_send_raw_transaction",
            (tx,),
        )
        .await;

        self.process_call_result(res.map(|val| val.0))
    }
}

thread_local! {
    static NONCE_CELL: RefCell<StableCell<U256>> = {
        RefCell::new(StableCell::new(NONCE_MEMORY_ID, U256::one())
            .expect("stable memory nonce initialization failed"))
    };
}
