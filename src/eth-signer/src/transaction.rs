use alloy::consensus::SignableTransaction;
use alloy::network::{TransactionBuilder as AlloyTransactionBuilder, TxSignerSync};
use alloy::rpc::types::{Transaction as AlloyRpcTransaction, TransactionRequest};
use alloy::signers::k256::ecdsa::SigningKey;
use did::error::EvmError;
use did::hash::H160;
use did::integer::U256;
use did::transaction::{calculate_tx_hash, Signature as DidSignature, Transaction as DidTransaction};

use crate::LocalWallet;

/// Method to create a transaction signature
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SigningMethod<'a> {
    // Do not sign transaction.
    // Could be used only for the cases when transactions isn't applied
    None,
    // Precalculated signature
    // Could be used only for the cases when the transaction is executed ReadOnly
    Signature(DidSignature),
    /// Use signing key to generate signature in `calculate_hash_and_build` method
    SigningKey(&'a SigningKey),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionBuilder<'a, 'b> {
    pub from: &'a H160,
    pub to: Option<H160>,
    pub nonce: U256,
    pub value: U256,
    pub gas: U256,
    pub gas_price: U256,
    pub input: Vec<u8>,
    pub signature: SigningMethod<'b>,
    pub chain_id: u64,
}

impl<'a, 'b> TransactionBuilder<'a, 'b> {
    /// Creates a new transaction with the expected hash
    pub fn calculate_hash_and_build(self) -> Result<DidTransaction, EvmError> {

        match self.signature.clone() {
            SigningMethod::None => {
                Ok(self.build_did_tx(DidSignature::default()))
            }
            SigningMethod::Signature(signature) => {
                Ok(self.build_did_tx(signature))
            }
            SigningMethod::SigningKey(key) => {
                let wallet =
                    LocalWallet::new_with_credential((*key).clone(), self.from.0, Some(self.chain_id));

                    let transaction = TransactionRequest::default()
                        .with_from(self.from.0)
                        .with_nonce(self.nonce.0.to())
                        .with_gas_price(self.gas_price.0.to())
                        .with_value(self.value.0)
                        .with_gas_limit(self.gas.0.to())
                        .with_chain_id(self.chain_id)
                        .with_input(alloy::primitives::Bytes::from(self.input))
                        .with_kind(self.to.map(|to| to.0.into()).into());

                    let REMOVE_UNWRAP = 0;
                    let tx = transaction.build_typed_tx().unwrap();

                    let mut tx = tx.legacy().cloned().unwrap();
                    let signature = wallet.sign_transaction_sync(&mut tx).unwrap();

                    let signed = tx.into_signed(signature);
                    let transaction: DidTransaction = AlloyRpcTransaction{
                        inner: signed.into(),
                        from: self.from.0,
                        block_hash: None,
                        block_number: None,
                        transaction_index: None,
                        effective_gas_price: None,
                    }.into();

                    // transaction.chain_id = Some(self.chain_id.into());
                    
                    Ok(transaction)

            }
        }

    }

    fn build_did_tx(self, signature: DidSignature) -> DidTransaction {
        let mut transaction = DidTransaction {
            to: self.to,
            from: self.from.clone(),
            nonce: self.nonce,
            value: self.value,
            gas: self.gas,
            gas_price: Some(self.gas_price),
            input: self.input.into(),
            chain_id: Some(self.chain_id.into()),
            transaction_type: Some(0u64.into()),
            r: signature.r,
            s: signature.s,
            v: signature.v,
            block_hash: None,
            block_number: None,
            transaction_index: None,
            access_list: None,
            max_priority_fee_per_gas: None,
            max_fee_per_gas: None,
            hash: Default::default()
        };
        transaction.hash = calculate_tx_hash(&transaction);
        transaction
    }
}

#[cfg(test)]
mod test {

    use alloy::signers::utils::secret_key_to_address;
    use did::U64;

    use super::*;

    #[test]
    fn test_build_transaction_with_empty_signature() {
        let transaction_builder = TransactionBuilder {
            from: &H160::from_slice(&[2u8; 20]),
            to: None,
            nonce: U256::zero(),
            value: U256::zero(),
            gas: 10_000u64.into(),
            gas_price: 20_000u64.into(),
            input: Vec::new(),
            signature: SigningMethod::None,
            chain_id: 31540,
        };
        let tx = transaction_builder.calculate_hash_and_build().unwrap();

        assert_eq!(tx.v, U64::zero());
        assert_eq!(tx.r, U256::zero());
        assert_eq!(tx.s, U256::zero());
        assert_eq!(tx.chain_id, Some(31540u64.into()));
    }

    #[test]
    fn test_build_transaction_with_fixed_signature() {
        let transaction_builder = TransactionBuilder {
            from: &H160::from_slice(&[2u8; 20]),
            to: None,
            nonce: U256::zero(),
            value: U256::zero(),
            gas: 10_000u64.into(),
            gas_price: 20_000u64.into(),
            input: Vec::new(),
            signature: SigningMethod::Signature(DidSignature {
                r: 1u64.into(),
                s: 2u64.into(),
                v: 0u64.into(),
            }),
            chain_id: 31541,
        };
        let tx = transaction_builder.calculate_hash_and_build().unwrap();

        assert_eq!(tx.v, U64::from(0u64));
        assert_eq!(tx.r, U256::from(1u64));
        assert_eq!(tx.s, U256::from(2u64));
        assert_eq!(tx.chain_id, Some(31541u64.into()));
    }

    // #[test]
    // fn test_build_transaction_with_signing_key() {
    //     let key = SigningKey::random(&mut rand::thread_rng());
    //     let from = secret_key_to_address(&key);
    //     let chain_id = 31540;
    //     let transaction_builder = TransactionBuilder {
    //         from: &from.into(),
    //         to: None,
    //         nonce: U256::zero(),
    //         value: U256::zero(),
    //         gas: 10_000u64.into(),
    //         gas_price: Some(20_000u64.into()),
    //         input: Vec::new(),
    //         signature: SigningMethod::SigningKey(&key),
    //         chain_id,
    //     };

    //     let tx = transaction_builder
    //         .calculate_hash_and_build()
    //         .unwrap();
    //     let typed_tx: TypedTransaction = (&tx).into();
    //     let wallet = LocalWallet::new_with_credential(key, from, Some(chain_id));
    //     let signature = wallet.sign_transaction_sync(&typed_tx).unwrap();

    //     assert_eq!(tx.v, signature.v.into());
    //     assert_eq!(tx.r, signature.r);
    //     assert_eq!(tx.s, signature.s);
    //     assert_eq!(tx.chain_id, Some(chain_id.into()));
    // }

    // #[test]
    // fn test_build_transaction_with_signing_key_should_include_chain_id() {
    //     let key = SigningKey::random(&mut rand::thread_rng());
    //     let from = secret_key_to_address(&key);
    //     let chain_id = 31540;
    //     let transaction_builder = TransactionBuilder {
    //         from: &from.into(),
    //         to: None,
    //         nonce: U256::zero(),
    //         value: U256::zero(),
    //         gas: 10_000u64.into(),
    //         gas_price: Some(20_000u64.into()),
    //         input: Vec::new(),
    //         signature: SigningMethod::SigningKey(&key),
    //         chain_id,
    //     };

    //     let tx: ethers_core::types::Transaction = transaction_builder
    //         .calculate_hash_and_build()
    //         .unwrap()
    //         .into();
    //     let mut typed_tx: TypedTransaction = (&tx).into();
    //     typed_tx.set_chain_id(chain_id + 1);
    //     let wallet = LocalWallet::new_with_credential(key, from, Some(chain_id));
    //     let signature_with_different_chain_id = wallet.sign_transaction_sync(&typed_tx).unwrap();

    //     assert_ne!(tx.v, signature_with_different_chain_id.v.into());
    //     assert_ne!(tx.r, signature_with_different_chain_id.r);
    //     assert_ne!(tx.s, signature_with_different_chain_id.s);
    //     assert_eq!(tx.chain_id, Some(chain_id.into()));
    // }

    #[test]
    fn test_build_transaction_should_have_recoverable_from() {
        let key = SigningKey::random(&mut rand::thread_rng());
        let from = secret_key_to_address(&key);
        let chain_id = 31540;
        let transaction_builder = TransactionBuilder {
            from: &from.into(),
            to: None,
            nonce: U256::zero(),
            value: U256::from(2_u64),
            gas: 10_000u64.into(),
            gas_price: 20_000u64.into(),
            input: [1, 2, 3].to_vec(),
            signature: SigningMethod::SigningKey(&key),
            chain_id,
        };

        let tx = alloy::rpc::types::Transaction::from(transaction_builder
            .calculate_hash_and_build()
            .unwrap());
        // let recovered_from = primitive_signature.recover_address_from_prehash(&tx.signature_hash()).unwrap();
        // assert_eq!(recovered_from, from);
        let recovered_from = tx.inner.signature().recover_address_from_prehash(&tx.inner.signature_hash()).unwrap();

        // let recovered_from = tx.recover_from().unwrap();
        assert_eq!(from, recovered_from);
    }
}
