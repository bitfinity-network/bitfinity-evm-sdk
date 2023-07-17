use std::cell::RefCell;
use std::rc::Rc;

use candid::{CandidType, Deserialize, Principal};
use did::sign_strategy::{ManagementCanisterSigner, SigningKeyId, TransactionSigner};
use did::transaction::{SigningMethod, TransactionBuilder};
use did::{BlockNumber, Transaction, TransactionReceipt, H160};
use ethers_core::k256::ecdsa::SigningKey;
use ethers_core::types::transaction::eip2718::TypedTransaction;
use ethers_core::utils;
use ic_canister::{generate_idl, init, update, Canister, Idl, PreUpdate};
use ic_exports::ic_ic00_types::DerivationPath;
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
    pub async fn test_send_raw_transaction_signed_with_signing_key(
        &mut self,
    ) -> (Transaction, TransactionReceipt) {
        let (chain_id, client) = {
            let state = self.state.borrow();
            let chain_id = state.chain_id;
            let client = iceth_client::Client::new(state.iceth, state.url.clone());
            (chain_id, client)
        };

        assert_eq!(chain_id, client.eth_get_chain_id().await.unwrap());

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
            .get_transaction_by_hash(tx_hash)
            .await
            .unwrap()
            .unwrap()
            .into();

        (tx, receipt)
    }

    #[update]
    pub async fn test_send_raw_transaction_signed_with_management_canister(
        &mut self,
    ) -> (Transaction, TransactionReceipt) {
        let (chain_id, client) = {
            let state = self.state.borrow();
            let chain_id = state.chain_id;
            let client = iceth_client::Client::new(state.iceth, state.url.clone());
            (chain_id, client)
        };

        assert_eq!(chain_id, client.eth_get_chain_id().await.unwrap());

        let signer = ManagementCanisterSigner::new(
            SigningKeyId::Dfx,
            DerivationPath::new(vec![chain_id.to_be_bytes().to_vec()]),
        );
        let address = signer.get_address().await.unwrap();

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
            signature: SigningMethod::None,
            chain_id,
        }
        .calculate_hash_and_build()
        .unwrap();
        let tx = ethers_core::types::Transaction::from(tx);
        let typed_tx: TypedTransaction = (&tx).into();

        let signature: ethers_core::types::Signature =
            signer.sign_transaction(&typed_tx).await.unwrap().into();

        let recovered = signature.recover(typed_tx.sighash()).unwrap();
        assert_eq!(recovered, tx.from);

        let rlp = typed_tx.rlp_signed(&signature);
        let tx_hash = client.send_raw_transaction(rlp.into()).await.unwrap();

        let receipt = client
            .get_transaction_receipt(&tx_hash)
            .await
            .unwrap()
            .unwrap();

        let tx = client
            .get_transaction_by_hash(tx_hash)
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
