use std::future::Future;
use std::sync::Arc;

use evm_block_extractor::database::{postgres_db_client::PostgresDbClient, DatabaseClient};
use sqlx::{postgres::PgConnectOptions, PgPool};
use testcontainers::testcontainers::{clients::Cli, Container};

mod client;
mod tests;

async fn test_with_clients<T: Fn(Arc<dyn DatabaseClient>) -> F, F: Future<Output = ()>>(test: T) {
    let docker = Cli::default();

    println!("----------------------------------");
    println!("Running test with PostgresDbClient");
    println!("----------------------------------");
    let (postgres_client, _node) = new_postgres_db_client(&docker).await;
    test(postgres_client).await;

    println!("----------------------------------");
    println!("Running test with BigQueryDbClient");
    println!("----------------------------------");
    let (bigquery_client, _node, _temp_file, _auth) = client::new_bigquery_client(&docker).await;
    test(bigquery_client).await;
}

async fn new_postgres_db_client(
    docker: &Cli,
) -> (Arc<dyn DatabaseClient>, Container<'_, testcontainers::postgres::Postgres>) {
    let node = docker.run(testcontainers::postgres::Postgres::default());

    let options = PgConnectOptions::new()
        .username("postgres")
        .password("postgres")
        .database("postgres")
        .host("127.0.0.1")
        .port(node.get_host_port_ipv4(5432));

    let pool = PgPool::connect_with(options).await.unwrap();
    let db_client = Arc::new(PostgresDbClient::new(pool));
    (db_client, node)
}
