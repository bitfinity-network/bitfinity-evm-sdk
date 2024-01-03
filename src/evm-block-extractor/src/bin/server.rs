use clap::{arg, Parser};
use evm_block_extractor::rpc::{EthImpl, EthServer};
use evm_block_extractor::storage_clients::gcp_big_query::BigQueryBlockChain;
use jsonrpsee::server::Server;
use jsonrpsee::RpcModule;

#[derive(Debug, Clone, Parser)]
pub struct ServerConfig {
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
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logger
    init_logger()?;
    let args = ServerConfig::parse();

    // Check if the dataset ID is valid
    if args.dataset_id != "testnet" && args.dataset_id != "mainnet" {
        return Err(anyhow::anyhow!(
            "Invalid dataset ID. The dataset ID can be one of the following: testnet, mainnet"
        ));
    }

    let server = Server::builder().build(args.server_address).await?;

    let db = BigQueryBlockChain::new(args.dataset_id, args.sa_key).await?;

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

    handle.stop()?;

    log::info!("Server stopped gracefully");

    Ok(())
}

fn init_logger() -> anyhow::Result<()> {
    let env = env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info");

    env_logger::Builder::from_env(env)
        .format_timestamp_millis()
        .init();

    Ok(())
}
