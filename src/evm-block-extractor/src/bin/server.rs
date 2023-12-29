use std::sync::Arc;

use clap::{arg, Parser};
use evm_block_extractor::rpc::{EthImpl, EthServer};
use evm_block_extractor::storage_clients::gcp_big_query::BigQueryBlockChain;
use jsonrpsee::server::Server;
use jsonrpsee::RpcModule;
use tokio::sync::oneshot;

#[derive(Debug, Clone, Parser)]
pub struct ServerConfig {
    /// The dataset ID of the BigQuery table
    #[arg(long = "dataset-id", short('d'))]
    pub dataset_id: String,

    /// Server address
    #[arg(long = "server-address", short('s'), default_value = "127.0.0.1:8080")]
    pub server_address: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logger
    init_logger()?;
    let args = ServerConfig::parse();

    let server = Server::builder().build(args.server_address).await?;

    let db = BigQueryBlockChain::new(args.dataset_id).await?;

    let eth = EthImpl::new(Arc::new(Box::new(db)));

    let mut module = RpcModule::new(());

    module.merge(EthServer::into_rpc(eth))?;

    log::info!("Server started on {}", server.local_addr()?);

    let handle = server.start(module);

    let (stopped_snd, stopped_recv) = oneshot::channel();

    tokio::spawn(graceful_shutdown(stopped_snd));
    stopped_recv.await?;
    handle.stop()?;

    log::info!("Server stopped gracefully");

    Ok(())
}

async fn graceful_shutdown(stopped_snd: oneshot::Sender<()>) {
    match tokio::signal::ctrl_c().await {
        Ok(_) => {
            if stopped_snd.send(()).is_err() {
                log::error!("Failed to send shutdown signal");
            }
        }
        Err(err) => log::error!("Failed to listen for shutdown signal: {err}"),
    }
}

fn init_logger() -> anyhow::Result<()> {
    let env = env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info");

    env_logger::Builder::from_env(env)
        .format_timestamp_millis()
        .init();

    Ok(())
}
