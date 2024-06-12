use alloy_consensus::{SignableTransaction, TypedTransaction};

/// Sets the chain id of the transaction
pub fn set_chain_id(tx: &mut TypedTransaction, chain_id: u64) {
    match tx {
        TypedTransaction::Legacy(ref mut tx) => {
            tx.chain_id = Some(chain_id);
        }
        TypedTransaction::Eip2930(ref mut tx) => {
            tx.chain_id = chain_id;
        }
        TypedTransaction::Eip1559(ref mut tx) => {
            tx.chain_id = chain_id;
        }
        TypedTransaction::Eip4844(ref mut tx) => {
            tx.set_chain_id(chain_id);
        }
    }
}