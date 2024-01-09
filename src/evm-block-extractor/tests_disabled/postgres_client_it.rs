use ethers_core::types::{Block, Transaction, TransactionReceipt, H256};
use evm_block_extractor::storage_clients::{postgres_client::PostgresBlockchain, BlockChainDB};
use sqlx::{postgres::PgConnectOptions, PgPool, Row};
use testcontainers::testcontainers::{clients::Cli, Container};

#[tokio::test]
async fn test_postgres_docker() {
    let docker = Cli::default();
    let (pool, _node) = new_postgres_pool(&docker).await;

    // container is up, we can use it
    let row: i32 = sqlx::query("SELECT 1 + 1")
        .fetch_one(&pool)
        .await
        .and_then(|row| row.try_get(0))
        .unwrap();

    assert_eq!(row, 2);
}

#[tokio::test]
async fn test_batch_insertion_of_blocks_and_receipts_retrieval_in_bq() {
    let docker = Cli::default();
    let (pool, _node) = new_postgres_pool(&docker).await;

    let blockchain = Box::new(PostgresBlockchain::new(pool));
    blockchain.init().await.unwrap();

    let mut blocks = Vec::new();

    for i in 1..=10 {
        let dummy_block: Block<Transaction> = ethers_core::types::Block {
            number: Some(ethers_core::types::U64::from(i)),
            hash: Some(H256::random()),
            ..Default::default()
        };

        blocks.push(dummy_block);
    }

    let mut receipts = Vec::new();

    for _ in 1..=10 {
        let tx_hash = H256::random();
        let dummy_receipt: TransactionReceipt = ethers_core::types::TransactionReceipt {
            transaction_hash: tx_hash,
            ..Default::default()
        };

        receipts.push(dummy_receipt);
    }

    blockchain
        .insert_blocks_and_receipts(&blocks, &receipts)
        .await
        .unwrap();

    let block = blockchain.get_block_by_number(1).await.unwrap();

    assert_eq!(block.number.unwrap().as_u64(), 1);

    let receipt = blockchain
        .get_transaction_receipt(receipts[0].transaction_hash)
        .await
        .unwrap();

    assert_eq!(receipt.transaction_hash, receipts[0].transaction_hash);

    let block = blockchain.get_block_by_number(10).await.unwrap();

    assert_eq!(block.number.unwrap().as_u64(), 10);

    let receipt = blockchain
        .get_transaction_receipt(receipts[9].transaction_hash)
        .await
        .unwrap();

    assert_eq!(receipt.transaction_hash, receipts[9].transaction_hash);
}

#[tokio::test]
async fn test_retrieval_of_latest_and_oldest_block_number() {
    let docker = Cli::default();
    let (pool, _node) = new_postgres_pool(&docker).await;

    let blockchain = Box::new(PostgresBlockchain::new(pool));
    blockchain.init().await.unwrap();

    for i in 1..=10 {
        let dummy_block: Block<Transaction> = ethers_core::types::Block {
            number: Some(ethers_core::types::U64::from(i)),
            hash: Some(H256::random()),
            ..Default::default()
        };

        blockchain
            .insert_blocks_and_receipts(&[dummy_block], &[])
            .await
            .unwrap();
    }

    let latest_block_number = blockchain.get_latest_block_number().await.unwrap();

    assert_eq!(latest_block_number, 10);

    let earliest_block_number = blockchain.get_earliest_block_number().await.unwrap();

    assert_eq!(earliest_block_number, 1);
}

#[tokio::test]
async fn test_init_idempotency() {
    let docker = Cli::default();
    let (pool, _node) = new_postgres_pool(&docker).await;

    let blockchain = Box::new(PostgresBlockchain::new(pool));
    blockchain.init().await.unwrap();

    // Add a block
    let dummy_block: Block<Transaction> = ethers_core::types::Block {
        number: Some(ethers_core::types::U64::from(1)),
        hash: Some(H256::random()),
        ..Default::default()
    };

    assert!(blockchain
        .insert_blocks_and_receipts(&[dummy_block], &[])
        .await
        .is_err());

    // First initialization - creates tables
    blockchain.init().await.unwrap();

    // Add a block
    let dummy_block: Block<Transaction> = ethers_core::types::Block {
        number: Some(ethers_core::types::U64::from(1)),
        hash: Some(H256::random()),
        ..Default::default()
    };

    assert!(blockchain
        .insert_blocks_and_receipts(&[dummy_block], &[])
        .await
        .is_ok());

    assert!(blockchain.init().await.is_ok());

    // Retrieve the block
    let block = blockchain.get_block_by_number(1).await.unwrap();

    assert_eq!(block.number.unwrap().as_u64(), 1);
}

async fn new_postgres_pool(
    docker: &Cli,
) -> (PgPool, Container<'_, testcontainers::postgres::Postgres>) {
    let node = docker.run(testcontainers::postgres::Postgres::default());

    let options = PgConnectOptions::new()
        .username("postgres")
        .password("postgres")
        .database("postgres")
        .host("127.0.0.1")
        .port(node.get_host_port_ipv4(5432));

    let pool = PgPool::connect_with(options).await.unwrap();
    (pool, node)
}
