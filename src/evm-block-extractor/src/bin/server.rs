use clap::{arg, Parser};
use evm_block_extractor::config::Database;
use evm_block_extractor::rpc::{EthImpl, EthServer};
use jsonrpsee::server::Server;
use jsonrpsee::RpcModule;

#[derive(Debug, Clone, Parser)]
pub struct ServerConfig {
    /// Server address
    #[arg(long = "server-address", short('s'), default_value = "127.0.0.1:8080")]
    pub server_address: String,

    /// Log level (default: info, options: trace, debug, info, warn, error)
    #[arg(long, default_value = "info")]
    pub log_level: String,

    #[command(subcommand)]
    command: Database,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logger
    let args = ServerConfig::parse();

    init_logger(args.log_level)?;

    let server = Server::builder().build(args.server_address).await?;

    let db_client = args.command.build_client().await?;
    db_client.init().await?;

    let eth = EthImpl::new(db_client);

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
