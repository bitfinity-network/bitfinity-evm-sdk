use alloy::{consensus::SignableTransaction, network::TransactionBuilder, primitives::{Address, B160, U256}, rpc::types::TransactionRequest};
use candid::Principal;
use eth_signer::ic_sign::{DerivationPath, IcSigner, SigningKeyId};
use ic_canister::{generate_idl, update, Canister, Idl, PreUpdate};

#[derive(Canister)]
pub struct TestCanister {
    #[id]
    id: Principal,
}

impl PreUpdate for TestCanister {}

impl TestCanister {
    /// Signs and recovers two different transactions and two different digests.
    #[update]
    pub async fn sign_and_check(&self) {
        let pubkey = IcSigner
            .public_key(SigningKeyId::PocketIc, DerivationPath::default())
            .await
            .unwrap();
        let from = IcSigner.pubkey_to_address(&pubkey).unwrap();

        let mut tx = TransactionRequest::default()
            .with_from(from.into())
            .with_to(Address::ZERO)
            .with_value(U256::from(10u64))
            .with_chain_id(355113)
            .with_nonce(0)
            .with_gas_price(10)
            .with_gas_limit(53000)
            .build_typed_tx().unwrap().legacy().cloned().unwrap();

        let signature = IcSigner
            .sign_transaction(
                &mut tx,
                &pubkey,
                SigningKeyId::PocketIc,
                DerivationPath::default(),
            )
            .await
            .unwrap();

            let recovered_from = signature.recover_from(&tx.signature_hash().into()).unwrap();
            assert_eq!(recovered_from.0, from);

        // Assert the chain ID is correctly encoded in the signature
        {
            // let tx_bytes = tx.rlp_signed(&signature);
    
            // let decoded_tx = Transaction::decode(&Rlp::new(&tx_bytes)).unwrap();
            // assert_eq!(decoded_tx.chain_id.unwrap().as_u64(), 355113);
        }

        let mut tx = TransactionRequest::default()
            .with_from(from.into())
            .with_to(Address::ZERO)
            .with_value(U256::from(10))
            .with_chain_id(355113)
            .with_nonce(1)
            .with_gas_price(10)
            .with_gas_limit(53000)
            .build_typed_tx().unwrap().legacy().cloned().unwrap();

        let signature = IcSigner
            .sign_transaction(
                &mut tx,
                &pubkey,
                SigningKeyId::PocketIc,
                DerivationPath::default(),
            )
            .await
            .unwrap();

        let recovered_from = signature.recover_from(&tx.signature_hash().into()).unwrap();
        assert_eq!(recovered_from.0, from);

        let digest = [42u8; 32];
        let signature = IcSigner
            .sign_digest(
                digest,
                &pubkey,
                SigningKeyId::PocketIc,
                DerivationPath::default(),
            )
            .await
            .unwrap();

        let recovered_from = signature.recover_from(&digest.into()).unwrap();
        assert_eq!(recovered_from.0, from);

        let digest = [43u8; 32];
        let signature = IcSigner
            .sign_digest(
                digest,
                &pubkey,
                SigningKeyId::PocketIc,
                DerivationPath::default(),
            )
            .await
            .unwrap();

        let recovered_from = signature.recover_from(&digest.into()).unwrap();
        assert_eq!(recovered_from.0, from);
    }

    /// Important: This function must be added to the canister to provide the idl.
    pub fn idl() -> Idl {
        generate_idl!()
    }
}
