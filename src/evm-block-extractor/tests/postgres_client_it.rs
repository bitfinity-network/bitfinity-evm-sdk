use sqlx::{postgres::PgConnectOptions, PgPool, Row};
use testcontainers::testcontainers::clients::Cli;


#[tokio::test]
async fn test_postgres_docker() {
        // startup the module
        let docker = Cli::default();
        let node = docker.run(testcontainers::postgres::Postgres::default());

        let options = PgConnectOptions::new()
        .username("postgres")
        .password("postgres")
        .database("postgres")
        .host("127.0.0.1")
        .port(node.get_host_port_ipv4(5432));

        let pool = PgPool::connect_with(options).await.unwrap();

        // container is up, you can use it
        let row: i32 = sqlx::query("SELECT 1 + 1")
            .fetch_one(&pool)
            .await
            .and_then(|row| row.try_get(0))
            .unwrap();

        assert_eq!(row, 2);
}
