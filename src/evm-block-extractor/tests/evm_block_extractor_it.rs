use std::future::Future;
use std::sync::Arc;

use evm_block_extractor::config::Database;
use evm_block_extractor::database::DatabaseClient;
use testcontainers::testcontainers::clients::Cli;
use testcontainers::testcontainers::Container;

mod tests;

async fn test_with_clients<T: Fn(Arc<dyn DatabaseClient>) -> F, F: Future<Output = ()>>(test: T) {
    let _ = env_logger::Builder::new().parse_filters("info").try_init();

    let docker = Cli::default();

    println!("----------------------------------");
    println!("Running test with PostgresDbClient");
    println!("----------------------------------");
    let (postgres_client, _node) = new_postgres_db_client(&docker).await;
    test(postgres_client).await;
}

async fn new_postgres_db_client(
    docker: &Cli,
) -> (
    Arc<dyn DatabaseClient>,
    Container<'_, testcontainers::postgres::Postgres>,
) {
    let node = docker.run(testcontainers::postgres::Postgres::default());

    let db = Database::Postgres {
        username: "postgres".to_string(),
        password: "postgres".to_string(),
        database_name: "postgres".to_string(),
        database_url: "127.0.0.1".to_owned(),
        database_port: node.get_host_port_ipv4(5432),
        require_ssl: false,
    };

    (db.build_client().await.unwrap(), node)
}
