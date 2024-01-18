use std::sync::Arc;

use clap::Subcommand;
use sqlx::postgres::{PgConnectOptions, PgSslMode};
use sqlx::PgPool;

use crate::database::big_query_db_client::BigQueryDbClient;
use crate::database::postgres_db_client::PostgresDbClient;
use crate::database::DatabaseClient;

#[derive(Subcommand, Debug, Clone)]
pub enum Database {
    #[command(name = "--bigquery")]
    BigQuery {
        /// The project ID of the BigQuery table
        #[arg(long = "project-id", short('p'), default_value = "bitfinity-evm")]
        project_id: String,

        /// The dataset ID of the BigQuery table
        /// The dataset ID can be one of the following:
        /// - `testnet`
        /// - `mainnet`
        #[arg(long = "dataset-id", short('d'))]
        dataset_id: String,

        /// The service account key in JSON format
        #[arg(long = "sa-key", short('k'), env = "GCP_BLOCK_EXTRACTOR_SA_KEY")]
        sa_key: String,
    },
    #[command(name = "--postgres")]
    Postgres {
        /// The username of the Postgres database
        #[arg(long)]
        username: String,
        /// The password of the Postgres database
        #[arg(long)]
        password: String,
        /// The name of the Postgres database
        #[arg(long)]
        database_name: String,
        /// The host of the Postgres database
        #[arg(long)]
        database_url: String,
        /// The port of the Postgres database
        #[arg(long)]
        database_port: u16,
    },
}

impl Database {
    /// Build a database client based on the database type
    pub async fn build_client(self) -> anyhow::Result<Arc<dyn DatabaseClient>> {
        match self {
            Database::BigQuery {
                project_id,
                dataset_id,
                sa_key,
            } => {
                log::info!("Use BigQuery database");
                log::info!("- project-id: {}", project_id);
                log::info!("- dataset-id: {}", dataset_id);

                if dataset_id != "testnet" && dataset_id != "mainnet" {
                    return Err(anyhow::anyhow!(
                        "Invalid dataset ID. The dataset ID can be one of the following: testnet, mainnet"
                    ));
                }
                let client = BigQueryDbClient::new(project_id, dataset_id, sa_key).await?;
                Ok(Arc::new(client))
            }
            Database::Postgres {
                username,
                password,
                database_name: database,
                database_url: host,
                database_port: port,
            } => {
                log::info!("Use Postgres database");
                log::info!("- username: {}", username);
                log::info!("- database: {}", database);
                log::info!("- host: {}", host);
                log::info!("- port: {}", port);

                let options = PgConnectOptions::new()
                    .username(&username)
                    .password(&password)
                    .database(&database)
                    .host(&host)
                    .port(port)
                    .ssl_mode(PgSslMode::Require);

                let pool = PgPool::connect_with(options).await?;
                Ok(Arc::new(PostgresDbClient::new(pool)))
            }
        }
    }
}
