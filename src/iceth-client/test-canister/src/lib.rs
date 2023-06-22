use std::cell::RefCell;
use std::rc::Rc;

use candid::{CandidType, Deserialize, Principal};
use did::transaction::{SigningMethod, TransactionBuilder};
use did::{BlockNumber, Transaction, TransactionReceipt, H160, H256};
use ethers_core::k256::ecdsa::SigningKey;
use ethers_core::utils;
use ic_canister::{generate_idl, init, update, Canister, Idl, PreUpdate};
use ic_storage::stable::Versioned;
use ic_storage::IcStorage;

#[derive(CandidType, Deserialize, IcStorage)]
pub struct State {
    iceth: Principal,
    url: String,
    chain_id: u64,
}

impl Default for State {
    fn default() -> Self {
        Self {
            iceth: Principal::anonymous(),
            url: String::default(),
            chain_id: 0,
        }
    }
}

impl Versioned for State {
    type Previous = ();

    fn upgrade((): ()) -> Self {
        Self::default()
    }
}

#[derive(Canister)]
pub struct CounterCanister {
    #[id]
    id: Principal,
    #[state]
    state: Rc<RefCell<State>>,
}

impl PreUpdate for CounterCanister {}

impl CounterCanister {
    #[init]
    pub fn init(&mut self, state: State) {
        *self.state.borrow_mut() = state;
    }

    #[update]
    pub async fn test_send_raw_transaction(&mut self) -> (Transaction, TransactionReceipt) {
        let (chain_id, client) = {
            let state = self.state.borrow();
            let chain_id = state.chain_id;
            let client = ic_eth_client::Client::new(state.iceth, state.url.clone());
            (chain_id, client)
        };

        let (key, address) = Self::alice_wallet();

        let balance = client
            .get_balance(&address, BlockNumber::Latest)
            .await
            .unwrap();
        if balance < 10_000_000u64.into() {
            client
                .mint_evm_token(&address, 10_000_000_000_u64.into())
                .await
                .unwrap();
        };

        let gas_price = client.gas_price().await.unwrap();
        let nonce = client
            .get_transaction_count(&address, BlockNumber::Latest)
            .await
            .unwrap();
        let tx = TransactionBuilder {
            from: &address,
            to: Some(H160::zero()),
            nonce,
            value: 1000u64.into(),
            gas: 10_000_000u64.into(),
            gas_price: Some(gas_price),
            input: vec![],
            signature: SigningMethod::SigningKey(&key),
            chain_id,
        }
        .calculate_hash_and_build()
        .unwrap();
        let tx = ethers_core::types::Transaction::from(tx);
        let rlp = tx.rlp();
        let tx_hash = client.send_raw_transaction(rlp.into()).await.unwrap();

        let receipt = client
            .get_transaction_receipt(&tx_hash)
            .await
            .unwrap()
            .unwrap();

        let tx = client
            .get_transaction_by_hash(
                H256::from_hex_str(
                    "0x7388f7419e5f437b4c15b6bb61965e0f102cda568bd5e14ec7df72be2bc23393",
                )
                .unwrap(),
            )
            .await
            .unwrap()
            .unwrap()
            .into();

        (tx, receipt)
    }

    /// Important: This function must be added to the canister to provide the idl.
    pub fn idl() -> Idl {
        generate_idl!()
    }

    fn alice_wallet() -> (SigningKey, H160) {
        let key = SigningKey::from_slice(&[242; 32]).unwrap();
        let address = utils::secret_key_to_address(&key);
        (key, address.into())
    }
}
