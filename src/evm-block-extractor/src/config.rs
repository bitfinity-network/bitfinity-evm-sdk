use std::sync::Arc;

use clap::{Parser, Subcommand};
use sqlx::postgres::{PgConnectOptions, PgSslMode};
use sqlx::PgPool;

use crate::database::postgres_db_client::PostgresDbClient;
use crate::database::DatabaseClient;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Simple CLI parser for the EVM block extractor
#[derive(Parser, Debug, Clone)]
#[clap(
    version = VERSION,
    about = "A tool to extract EVM blocks and transactions and serve them through JSON RPC endpoints"
)]
pub struct ExtractorArgs {
    /// The server address to bind to serve JSON RPC requests
    #[arg(long = "server-address", short('s'), default_value = "0.0.0.0:8080")]
    pub server_address: String,

    /// The JSON-RPC URL of the remote EVMC instance from which to extract blocks.
    /// If missing or empty the block extracting task won't start.
    #[arg(long = "rpc-url", short('u'))]
    pub remote_rpc_url: String,

    /// Time in seconds to wait for a response from the EVMC
    #[arg(long, default_value = "60")]
    pub request_time_out_secs: u64,

    #[arg(long, default_value = "10")]
    pub rpc_batch_size: usize,

    /// Sets the logger [`EnvFilter`].
    /// Valid values: trace, debug, info, warn, error
    /// Example of a valid filter: "warn,my_crate=info,my_crate::my_mod=debug,[my_span]=trace".
    #[arg(long, default_value = "info")]
    pub log_filter: String,

    #[command(subcommand)]
    pub command: Database,

    /// Whether to reset the database when the blockchain state changes.
    /// This is useful for testing environments, but should not be used in production.
    #[arg(long, default_value = "false")]
    pub reset_db_on_state_change: bool,

    /// The interval in seconds at which the block extractor job should run
    #[arg(long, default_value = "120")]
    pub block_extractor_job_interval_seconds: u64,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Database {
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
        #[arg(long, default_value = "5432")]
        database_port: u16,
        /// Demand SSL connection
        #[arg(long, default_value = "false")]
        require_ssl: bool,
    },
}

impl Database {
    /// Build a database client based on the database type
    pub async fn build_client(self) -> anyhow::Result<Arc<dyn DatabaseClient>> {
        match self {
            Database::Postgres {
                username,
                password,
                database_name: database,
                database_url: host,
                database_port: port,
                require_ssl,
            } => {
                log::info!("Use Postgres database");
                log::info!("- username: {}", username);
                log::info!("- database: {}", database);
                log::info!("- host: {}", host);
                log::info!("- port: {}", port);
                log::info!("- require-ssl: {}", require_ssl);

                let ssl_mode = if require_ssl {
                    PgSslMode::Require
                } else {
                    PgSslMode::Prefer
                };

                let options = PgConnectOptions::new()
                    .username(&username)
                    .password(&password)
                    .database(&database)
                    .host(&host)
                    .port(port)
                    .ssl_mode(ssl_mode);

                let pool = PgPool::connect_with(options).await?;
                Ok(Arc::new(PostgresDbClient::new(pool)))
            }
        }
    }
}
