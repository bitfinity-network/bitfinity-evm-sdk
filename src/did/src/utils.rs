use crate::{Transaction, H160, U256};

/// Creates an ephemeral transaction to calculate the Proof of Work (PoW)
/// required by the EVM block confirmation endpoint.
/// This transaction is not meant to be used in a real blockchain.
///
pub fn block_confirmation_pow_transaction(
    from: H160,
    base_fee: Option<U256>,
    nonce: Option<U256>,
    gas_price: Option<U256>,
) -> Transaction {
    let base_fee = base_fee.unwrap_or_default();
    let gas_price = gas_price.unwrap_or(U256::from(1_u64) + base_fee);
    let nonce = nonce.unwrap_or(U256::from(0_u64));

    Transaction {
        from,
        to: Some(H160::zero()),
        value: U256::from(1_u64),
        gas: U256::from(23000_u64),
        gas_price: Some(gas_price),
        nonce,
        ..Default::default()
    }
}
