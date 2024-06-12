use alloy_consensus::{SignableTransaction, TypedTransaction};
use alloy_primitives::B256;

/// Sets the chain id of the transaction
pub fn set_chain_id(tx: &mut TypedTransaction, chain_id: u64) {
    match tx {
        TypedTransaction::Legacy(ref mut tx) => {
            tx.chain_id = Some(chain_id);
            tx.signature_hash();
        }
        TypedTransaction::Eip2930(ref mut tx) => {
            tx.chain_id = chain_id;
            tx.signature_hash();
        }
        TypedTransaction::Eip1559(ref mut tx) => {
            tx.chain_id = chain_id;
            tx.signature_hash();
        }
        TypedTransaction::Eip4844(ref mut tx) => {
            tx.set_chain_id(chain_id);
            tx.signature_hash();
        }
    }
}

/// Returns the transaction signature hash
pub fn transaction_signature_hash(tx: &TypedTransaction) -> B256 {
    match tx {
        TypedTransaction::Legacy(tx) => tx.signature_hash(),
        TypedTransaction::Eip2930(tx) => tx.signature_hash(),
        TypedTransaction::Eip1559(tx) => tx.signature_hash(),
        TypedTransaction::Eip4844(tx) => tx.signature_hash(),
    }
}