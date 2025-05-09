use std::sync::Arc;

use evm_block_extractor::config::Database;
use evm_block_extractor::database::postgres_db_client::PostgresDbClient;
use testcontainers::testcontainers::ContainerAsync;
use testcontainers::testcontainers::runners::AsyncRunner;

mod tests;

async fn test_with_clients<T: AsyncFn(Arc<PostgresDbClient>) -> ()>(test: T) {
    let _ = env_logger::Builder::new().parse_filters("info").try_init();
    println!("----------------------------------");
    println!("Running test with PostgresDbClient");
    println!("----------------------------------");
    let (postgres_client, _node) = new_postgres_db_client().await;
    test(postgres_client).await;
}

async fn new_postgres_db_client() -> (
    Arc<PostgresDbClient>,
    ContainerAsync<testcontainers::postgres::Postgres>,
) {
    let node = testcontainers::postgres::Postgres::default()
        .start()
        .await
        .unwrap();

    let db = Database::Postgres {
        username: "postgres".to_string(),
        password: "postgres".to_string(),
        database_name: "postgres".to_string(),
        database_url: "127.0.0.1".to_owned(),
        database_port: node.get_host_port_ipv4(5432).await.unwrap(),
        require_ssl: false,
    };

    (db.build_client().await.unwrap(), node)
}
