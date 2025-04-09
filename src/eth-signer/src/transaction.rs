use alloy::consensus::transaction::Recovered;
use alloy::consensus::{SignableTransaction, TxLegacy};
use alloy::network::TxSignerSync;
use alloy::rpc::types::Transaction as AlloyRpcTransaction;
use alloy::signers::k256::ecdsa::SigningKey;
use did::error::EvmError;
use did::hash::H160;
use did::integer::U256;
use did::transaction::{Signature as DidSignature, Transaction as DidTransaction};

use crate::LocalWallet;

/// Method to create a transaction signature
#[derive(Debug, Clone)]
pub enum SigningMethod<'a> {
    // Do not sign transaction.
    // Could be used only for the cases when transactions isn't applied.
    None,
    // Precalculated signature.
    // Could be used only for the cases when the transaction is executed ReadOnly.
    Signature(DidSignature),
    /// Use signing key to generate signature in `calculate_hash_and_build` method.
    /// Whenever possible use `LocalWallet` instead of `SigningKey` as it requires less allocations.
    SigningKey(&'a SigningKey),
    /// Use the wallet to generate signature in `calculate_hash_and_build` method.
    LocalWallet(&'a LocalWallet),
}

#[derive(Debug, Clone)]
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

impl TransactionBuilder<'_, '_> {
    /// Creates a new transaction with the expected hash
    pub fn calculate_hash_and_build(self) -> Result<DidTransaction, EvmError> {
        let mut transaction = TxLegacy {
            chain_id: Some(self.chain_id),
            nonce: self.nonce.0.to(),
            gas_price: self.gas_price.0.to(),
            gas_limit: self.gas.0.to(),
            to: self.to.map(|to| to.0).into(),
            value: self.value.0,
            input: alloy::primitives::Bytes::from(self.input),
        };

        let alloy_signature = match self.signature {
            SigningMethod::None => DidSignature::default().into(),
            SigningMethod::Signature(signature) => signature.into(),
            SigningMethod::SigningKey(key) => {
                let wallet = LocalWallet::new_with_credential(
                    (*key).clone(),
                    self.from.0,
                    Some(self.chain_id),
                );

                wallet
                    .sign_transaction_sync(&mut transaction)
                    .map_err(|err| EvmError::SignatureError(err.to_string()))?
            }
            SigningMethod::LocalWallet(wallet) => wallet
                .sign_transaction_sync(&mut transaction)
                .map_err(|err| EvmError::SignatureError(err.to_string()))?,
        };

        let signed = transaction.into_signed(alloy_signature);
        let transaction: DidTransaction = AlloyRpcTransaction {
            inner: Recovered::new_unchecked(signed.into(), self.from.0),
            block_hash: None,
            block_number: None,
            transaction_index: None,
            effective_gas_price: None,
        }
        .into();

        Ok(transaction)
    }
}

#[cfg(test)]
mod test {

    use alloy::consensus::TxEnvelope;
    use alloy::signers::k256::ecdsa::signature::hazmat::PrehashVerifier;
    use alloy::signers::utils::secret_key_to_address;

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

        // eip-155 legacy TX
        let expected_v = 31540u64 * 2 + 35;
        assert_eq!(tx.v.as_u64(), expected_v);
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
            signature: SigningMethod::Signature(
                DidSignature::new_from_rsv(1u64.into(), 2u64.into(), 1u64).unwrap(),
            ),
            chain_id: 31541,
        };
        let tx = transaction_builder.calculate_hash_and_build().unwrap();

        // eip-155 legacy TX
        let expected_v = 31541u64 * 2 + 35 + 1;
        assert_eq!(tx.v.as_u64(), expected_v);
        assert_eq!(tx.r, U256::from(1u64));
        assert_eq!(tx.s, U256::from(2u64));
        assert_eq!(tx.chain_id, Some(31541u64.into()));
    }

    #[test]
    fn test_build_transaction_with_signing_key() {
        let key = SigningKey::random(&mut rand::thread_rng());
        let from = secret_key_to_address(&key);
        let chain_id = 31550;
        let transaction_builder = TransactionBuilder {
            from: &from.into(),
            to: None,
            nonce: U256::zero(),
            value: U256::zero(),
            gas: 10_000u64.into(),
            gas_price: 20_000u64.into(),
            input: Vec::new(),
            signature: SigningMethod::SigningKey(&key),
            chain_id,
        };

        let tx = transaction_builder.calculate_hash_and_build().unwrap();

        assert_eq!(tx.chain_id, Some(chain_id.into()));

        let typed_tx = alloy::rpc::types::Transaction::try_from(tx.clone()).unwrap();

        let signature_hash = typed_tx.inner.signature_hash();
        let signature = typed_tx.inner.signature();

        // recover address from signature
        {
            let recovered_from = signature
                .recover_address_from_prehash(&signature_hash)
                .unwrap();
            assert_eq!(recovered_from, from);
        }

        // verify signature using the public key
        {
            let wallet = LocalWallet::new_with_credential(key, from, Some(chain_id));
            let verifying_key = wallet.credential().verifying_key();
            assert!(
                verifying_key
                    .verify_prehash(signature_hash.as_slice(), &signature.to_k256().unwrap())
                    .is_ok()
            );
        }

        // verify eip-155 signature
        {
            let signature = DidSignature::from(*signature);
            let expected_v = signature.v(did::transaction::TxChainInfo::LegacyTx {
                chain_id: Some(chain_id),
            });
            assert_eq!(tx.v.as_u64(), expected_v);
            assert_eq!(tx.r, signature.r);
            assert_eq!(tx.s, signature.s);
            assert_eq!(tx.chain_id, Some(chain_id.into()));
        }
    }

    #[test]
    fn test_build_transaction_with_signing_key_should_include_chain_id() {
        let key = SigningKey::random(&mut rand::thread_rng());
        let from = secret_key_to_address(&key);
        let chain_id = 31540;

        let tx_builder_1 = TransactionBuilder {
            from: &from.into(),
            to: None,
            nonce: U256::zero(),
            value: U256::zero(),
            gas: 10_000u64.into(),
            gas_price: 20_000u64.into(),
            input: Vec::new(),
            signature: SigningMethod::SigningKey(&key),
            chain_id,
        };

        let tx_1 = tx_builder_1.clone().calculate_hash_and_build().unwrap();

        let tx_builder_2 = TransactionBuilder {
            chain_id: chain_id + 1,
            ..tx_builder_1
        };

        let tx_2 = tx_builder_2.calculate_hash_and_build().unwrap();

        assert_ne!(tx_1.r, tx_2.r);
        assert_ne!(tx_1.s, tx_2.s);
        assert_ne!(tx_1.chain_id, tx_2.chain_id);
    }

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

        let tx = alloy::rpc::types::Transaction::try_from(
            transaction_builder.calculate_hash_and_build().unwrap(),
        )
        .unwrap();

        let recovered_from = tx
            .inner
            .signature()
            .recover_address_from_prehash(&tx.inner.signature_hash())
            .unwrap();

        assert_eq!(from, recovered_from);
    }

    #[test]
    fn test_build_transaction_is_protected_from_replay_attack() {
        let key = SigningKey::random(&mut rand::thread_rng());
        let from = secret_key_to_address(&key);
        let chain_id = 31540;

        let tx_1 = TransactionBuilder {
            from: &from.into(),
            to: None,
            nonce: U256::zero(),
            value: U256::zero(),
            gas: 10_000u64.into(),
            gas_price: 20_000u64.into(),
            input: Vec::new(),
            signature: SigningMethod::SigningKey(&key),
            chain_id,
        }
        .calculate_hash_and_build()
        .unwrap();

        let tx_1_envelop = TxEnvelope::try_from(tx_1.clone()).unwrap();
        let recovered_tx_1_from = tx_1_envelop.recover_signer().unwrap();

        // Replaying the same transaction with different chain_id
        let tx_2 = DidTransaction {
            chain_id: Some(U256::from(chain_id + 1u64)),
            ..tx_1
        };

        let tx_2_envelop = TxEnvelope::try_from(tx_2.clone()).unwrap();
        let recovered_tx_2_from = tx_2_envelop.recover_signer().unwrap();

        assert_ne!(recovered_tx_1_from, recovered_tx_2_from);
    }
}
