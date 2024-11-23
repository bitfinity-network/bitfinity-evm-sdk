use did::transaction::Bloom;
use did::{Block, Transaction, H160, H256, H64, U256, U64};
use eth_signer::transaction::{SigningMethod, TransactionBuilder};
use ethers_core::k256::ecdsa::SigningKey;

fn create_transaction(gas_price: Option<U256>, chain_id: u64) -> Transaction {
    TransactionBuilder {
        gas_price,
        signature: SigningMethod::SigningKey(&SigningKey::from_slice(&[4u8; 32]).unwrap()),
        from: &H160::from_slice(&[0u8; 20]),
        to: None,
        nonce: U256::zero(),
        value: U256::zero(),
        input: vec![],
        gas: 20u64.into(),
        chain_id,
    }
    .calculate_hash_and_build()
    .unwrap()
}

#[test]
fn test_block_rlp_serialization_roundtrip() {
    let chain_id = 31154;
    let block = Block::<Transaction> {
        author: H160::from_slice(&[3u8; 20]),
        number: U64::from(12u64),
        logs_bloom: Bloom(ethereum_types::Bloom::from_slice(&[4u8; 256])),
        nonce: H64::zero(),
        transactions: vec![create_transaction(Some(Default::default()), chain_id)],
        mix_hash: Default::default(), // during the serialization empty value is equivalent to the default
        hash: Default::default(),
        parent_hash: H256::from_slice(&[1u8; 32]),
        uncles_hash: H256::from_slice(&[4u8; 32]),
        state_root: H256::from_slice(&[5u8; 32]),
        transactions_root: H256::from_slice(&[6u8; 32]),
        receipts_root: H256::from_slice(&[7u8; 32]),
        gas_used: U256::from(20u64),
        gas_limit: U256::from(30u64),
        extra_data: Default::default(),
        timestamp: U256::from(40u64),
        difficulty: U256::from(50u64),
        total_difficulty: Default::default(),
        seal_fields: Vec::new(),
        uncles: Vec::new(),
        size: None,
        base_fee_per_gas: None,
    };

    let rlp_data = rlp::encode(&block);
    let recovered_block: Block<Transaction> = rlp::decode(&rlp_data).unwrap();

    assert_eq!(block, recovered_block);
}

#[test]
fn test_block_rlp_serialization_roundtrip_with_base_fee_per_gas() {
    let chain_id = 31154;
    let block = Block::<Transaction> {
        author: ethereum_types::H160::random().into(),
        number: U64::from(rand::random::<u64>()),
        logs_bloom: Bloom(ethereum_types::Bloom::from_slice(&[4u8; 256])),
        nonce: ethereum_types::H64::random().into(),
        transactions: vec![create_transaction(
            Some(U256::from(rand::random::<u64>())),
            chain_id,
        )],
        mix_hash: ethereum_types::H256::random().into(), // during the serialization empty value is equivalent to the default
        hash: Default::default(),
        parent_hash: ethereum_types::H256::random().into(),
        uncles_hash: ethereum_types::H256::random().into(),
        state_root: ethereum_types::H256::random().into(),
        transactions_root: ethereum_types::H256::random().into(),
        receipts_root: ethereum_types::H256::random().into(),
        gas_used: U256::from(rand::random::<u64>()),
        gas_limit: U256::from(rand::random::<u64>()),
        extra_data: Default::default(),
        timestamp: U256::from(rand::random::<u64>()),
        difficulty: U256::from(rand::random::<u64>()),
        total_difficulty: Default::default(),
        seal_fields: Vec::new(),
        uncles: Vec::new(),
        size: None,
        base_fee_per_gas: Some(U256::from(rand::random::<u64>())),
    };

    let rlp_data = rlp::encode(&block);
    let recovered_block: Block<Transaction> = rlp::decode(&rlp_data).unwrap();

    assert_eq!(block, recovered_block);
}
