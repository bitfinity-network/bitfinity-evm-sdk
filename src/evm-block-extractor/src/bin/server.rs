use clap::{arg, Parser};
use evm_block_extractor::database::big_query_db_client::BigQueryDbClient;
use evm_block_extractor::database::DatabaseClient;
use evm_block_extractor::rpc::{EthImpl, EthServer};
use jsonrpsee::server::Server;
use jsonrpsee::RpcModule;

#[derive(Debug, Clone, Parser)]
pub struct ServerConfig {
    /// The project ID of the BigQuery table
    #[arg(long = "project-id", short('p'), default_value = "bitfinity-evm")]
    project_id: String,

    /// The dataset ID of the BigQuery table
    /// The dataset ID can be one of the following:
    /// - `testnet`
    /// - `mainnet`
    #[arg(long = "dataset-id", short('d'))]
    pub dataset_id: String,

    /// Server address
    #[arg(long = "server-address", short('s'), default_value = "127.0.0.1:8080")]
    pub server_address: String,

    /// The service account key in JSON format
    #[arg(long = "sa-key", short('k'), env = "GCP_BLOCK_EXTRACTOR_SA_KEY")]
    pub sa_key: String,

    /// Log level (default: info, options: trace, debug, info, warn, error)
    #[arg(long, default_value = "info")]
    pub log_level: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logger
    let args = ServerConfig::parse();

    init_logger(args.log_level)?;

    // Check if the dataset ID is valid
    if args.dataset_id != "testnet" && args.dataset_id != "mainnet" {
        return Err(anyhow::anyhow!(
            "Invalid dataset ID. The dataset ID can be one of the following: testnet, mainnet"
        ));
    }

    let server = Server::builder().build(args.server_address).await?;

    let db = BigQueryDbClient::new(args.project_id, args.dataset_id, args.sa_key).await?;

    db.init().await?;

    let eth = EthImpl::new(db);

    let mut module = RpcModule::new(());

    module.merge(EthServer::into_rpc(eth))?;

    log::info!("Server started on {}", server.local_addr()?);

    let handle = server.start(module);

    match tokio::signal::ctrl_c().await {
        Ok(_) => {
            log::info!("Received shutdown signal");
        }
        Err(err) => log::error!("Failed to listen for shutdown signal: {err}"),
    }

    // Stop the server
    {
        handle.stop()?;
        handle.stopped().await;
    }

    log::info!("Server stopped gracefully");

    Ok(())
}

fn init_logger(log_level: String) -> anyhow::Result<()> {
    let level = log_level
        .parse::<log::LevelFilter>()
        .unwrap_or(log::LevelFilter::Info);

    env_logger::Builder::new().filter(None, level).try_init()?;

    Ok(())
}
