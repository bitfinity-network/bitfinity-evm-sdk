use crate::{Transaction, H160, U256};

pub fn pow_transaction(base_fee: Option<U256>) -> Transaction {
    let base_fee = base_fee.unwrap_or_default();
    Transaction {
        from: H160::from_slice(&[1; 20]),
        to: Some(H160::zero()),
        value: U256::from(1_u64),
        gas_price: Some(U256::from(1_u64) + base_fee),
        gas: U256::from(23000_u64),
        nonce: U256::from(0_u64),
        ..Default::default()
    }
}
