use did::{Transaction, H160, U256, U64};
use eth_signer::transaction::{SigningMethod, TransactionBuilder};

fn build_transaction(
    tx_type: Option<u64>,
    gas_price: Option<U256>,
    max_priority_fee_per_gas: Option<U256>,
    max_fee_per_gas: Option<U256>,
) -> Transaction {
    let mut tx = TransactionBuilder {
        from: &H160::from_slice(&[2u8; 20]),
        to: None,
        nonce: U256::zero(),
        value: U256::zero(),
        gas: 10_000u64.into(),
        gas_price: None,
        input: Vec::new(),
        signature: SigningMethod::None,
        chain_id: 31540,
    }
    .calculate_hash_and_build()
    .unwrap();

    match tx_type {
        Some(tx_type) if tx_type == 1 => {
            tx.transaction_type = Some(U64::from(1u64));
            tx.gas_price = gas_price;
        }
        Some(tx_type) if tx_type == 2 => {
            tx.transaction_type = Some(U64::from(2u64));
            tx.max_priority_fee_per_gas = max_priority_fee_per_gas;
            tx.max_fee_per_gas = max_fee_per_gas;
        }
        Some(_) => panic!("Invalid transaction type"),
        None => tx.gas_price = gas_price,
    }

    tx
}

#[test]
fn test_gas_cost_for_different_transaction_types() {
    let txns = vec![
        (
            build_transaction(Some(1), Some(20_000u64.into()), None, None),
            20_000u64.into(),
        ),
        (
            build_transaction(
                Some(2),
                None,
                Some(20_000u64.into()),
                Some(30_000u64.into()),
            ),
            30_000u64.into(),
        ),
        (
            build_transaction(None, Some(20_000u64.into()), None, None),
            20_000u64.into(),
        ),
        (
            build_transaction(None, Some(20_000u64.into()), None, None),
            20_000u64.into(),
        ),
    ];

    for (tx, expected_gas_cost) in txns {
        assert_eq!(tx.gas_cost(), expected_gas_cost);
    }
}

#[test]
fn test_max_priority_fee_or_gas_price_for_different_transaction_types() {
    let txns = vec![
        (
            build_transaction(Some(1), Some(20_000u64.into()), None, None),
            20_000u64.into(),
        ),
        (
            build_transaction(
                Some(2),
                None,
                Some(20_000u64.into()),
                Some(30_000u64.into()),
            ),
            20_000u64.into(),
        ),
        (
            build_transaction(None, Some(20_000u64.into()), None, None),
            20_000u64.into(),
        ),
        (
            build_transaction(None, Some(20_000u64.into()), None, None),
            20_000u64.into(),
        ),
    ];

    for (tx, expected_max_priority_fee_or_gas_price) in txns {
        assert_eq!(
            tx.max_priority_fee_or_gas_price(),
            expected_max_priority_fee_or_gas_price
        );
    }
}

#[test]
fn test_effective_gas_tip_for_different_transaction_types() {
    let base_per_gas: U256 = 20_000u64.into();

    let tx = build_transaction(Some(1), Some(30_000u64.into()), None, None);

    assert_eq!(
        tx.effective_gas_tip(Some(base_per_gas.clone())).unwrap(),
        10_000u64.into()
    );

    let tx = build_transaction(
        Some(2),
        None,
        Some(30_000u64.into()),
        Some(40_000u64.into()),
    );

    assert_eq!(
        tx.effective_gas_tip(Some(base_per_gas)).unwrap(),
        20_000u64.into()
    );

    let tx = build_transaction(None, Some(30_000u64.into()), None, None);

    assert_eq!(
        tx.effective_gas_tip(None).unwrap(),
        tx.max_priority_fee_or_gas_price()
    )
}
