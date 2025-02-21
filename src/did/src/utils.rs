use crate::{Transaction, H160, U256};

/// Creates an ephemeral transaction to calculate the Proof of Work (PoW)
/// required by the EVM block confirmation endpoint.
/// This transaction is not meant to be used in a real blockchain.
/// # Arguments
///
/// * `base_fee` - Optional base fee to be added to gas price. If None, defaults to 0
/// * `from` - The address of the sender of the transaction
///
pub fn block_confirmation_pow_transaction(from: H160, base_fee: Option<U256>) -> Transaction {
    let base_fee = base_fee.unwrap_or_default();
    Transaction {
        from,
        to: Some(H160::zero()),
        value: U256::from(1_u64),
        gas_price: Some(U256::from(1_u64) + base_fee),
        gas: U256::from(23000_u64),
        nonce: U256::from(0_u64),
        ..Default::default()
    }
}
