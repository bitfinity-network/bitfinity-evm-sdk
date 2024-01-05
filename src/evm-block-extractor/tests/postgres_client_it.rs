use ethers_core::types::{Block, Transaction, H256, TransactionReceipt};
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
async fn test_insertion_of_blocks_and_retrieval_in_bq() {

    let docker = Cli::default();
    let (pool, _node) = new_postgres_pool(&docker).await;

    let mut blockchain = Box::new(PostgresBlockchain::new(pool));
    blockchain.init().await.unwrap();

    let dummy_block: Block<Transaction> = ethers_core::types::Block {
        number: Some(ethers_core::types::U64::from(1)),
        ..Default::default()
    };

    blockchain.insert_block(&dummy_block).await.unwrap();

    let block = blockchain.get_block_by_number(1).await.unwrap();

    assert_eq!(block.number.unwrap().as_u64(), 1);

    let latest_block_number = blockchain.get_latest_block_number().await.unwrap();

    assert_eq!(latest_block_number, 1);
}

#[tokio::test]
async fn test_insertion_of_receipts_and_retrieval() {
    let docker = Cli::default();
    let (pool, _node) = new_postgres_pool(&docker).await;

    let mut blockchain = Box::new(PostgresBlockchain::new(pool));
    blockchain.init().await.unwrap();

    let tx_hash = H256::random();
    let dummy_receipt: TransactionReceipt = ethers_core::types::TransactionReceipt {
        transaction_hash: tx_hash,
        ..Default::default()
    };

    blockchain.insert_receipts(&[dummy_receipt]).await.unwrap();

    let receipt = blockchain.get_transaction_receipt(tx_hash).await.unwrap();

    assert_eq!(receipt.transaction_hash, tx_hash);
}

#[tokio::test]
async fn test_getting_block_range() {
    let docker = Cli::default();
    let (pool, _node) = new_postgres_pool(&docker).await;

    let mut blockchain = Box::new(PostgresBlockchain::new(pool));
    blockchain.init().await.unwrap();

    for i in 1..=10 {
        let dummy_block: Block<Transaction> = ethers_core::types::Block {
            number: Some(ethers_core::types::U64::from(i)),
            ..Default::default()
        };

        blockchain.insert_block(&dummy_block).await.unwrap();
    }

    let block_range = blockchain.get_blocks_in_range(1, 10).await.unwrap();

    assert_eq!(block_range, (1..=10).collect::<Vec<u64>>());

    let block_range = blockchain.get_blocks_in_range(1, 5).await.unwrap();

    assert_eq!(block_range, (1..=5).collect::<Vec<u64>>());
}

#[tokio::test]
async fn test_retrieval_of_latest_and_oldest_block_number() {
    let docker = Cli::default();
    let (pool, _node) = new_postgres_pool(&docker).await;

    let blockchain = Box::new(PostgresBlockchain::new(pool));
    blockchain.init().await.unwrap();

    let latest_block_number = blockchain.get_latest_block_number().await.unwrap();

    assert_eq!(latest_block_number, 10);

    let earliest_block_number = blockchain.get_earliest_block_number().await.unwrap();

    assert_eq!(earliest_block_number, 1);
}




async fn new_postgres_pool(docker: &Cli) -> (PgPool, Container<'_, testcontainers::postgres::Postgres>) {
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
