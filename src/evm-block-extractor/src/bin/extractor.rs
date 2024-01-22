use std::sync::Arc;

use clap::Parser;
use ethereum_json_rpc_client::reqwest::ReqwestClient;
use ethereum_json_rpc_client::EthJsonRcpClient;
use ethers_core::types::BlockNumber;
use evm_block_extractor::block_extractor::BlockExtractor;
use evm_block_extractor::config::Database;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const PACKAGE: &str = env!("CARGO_PKG_NAME");

/// Simple CLI program for Benchmarking BitFinity Network
#[derive(Parser, Debug)]
#[clap(
    version = VERSION,
    about = "A tool to extract EVM blocks and transactions and send them to a specified endpoint"
)]
struct Args {
    /// The JSON-RPC URL of the EVMC
    #[arg(long = "rpc-url", short('u'))]
    rpc_url: String,

    /// Time in seconds to wait for a response from the EVMC
    #[arg(long, default_value = "60")]
    request_time_out_secs: u64,

    #[arg(long, default_value = "10")]
    rpc_batch_size: usize,

    /// Log level (default: info, options: trace, debug, info, warn, error)
    #[arg(long, default_value = "info")]
    log_level: String,

    #[command(subcommand)]
    command: Database,

    /// Whether to reset the database when the blockchain state changes.
    /// This is useful for testing environments, but should not be used in production.
    #[arg(long, default_value = "false")]
    reset_db_on_state_change: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Initialize logger
    init_logger(args.log_level)?;

    log::info!("{PACKAGE}");
    log::info!("----------------------");
    log::info!("- rpc-url: {}", args.rpc_url);
    log::info!("- request_time_out_secs: {}", args.request_time_out_secs);
    log::info!("- reset_db_on_state_change: {}", args.reset_db_on_state_change);
    log::info!("----------------------");

    let evm_client = Arc::new(EthJsonRcpClient::new(ReqwestClient::new(args.rpc_url)));

    let earliest_block = evm_client
        .get_block_by_number(BlockNumber::Earliest)
        .await?;

    let db_client = args.command.build_client().await?;
    db_client
        .init(Some(earliest_block.into()), args.reset_db_on_state_change)
        .await?;

    let mut extractor = BlockExtractor::new(
        evm_client.clone(),
        args.request_time_out_secs,
        args.rpc_batch_size,
        db_client.clone(),
    );

    let end_block = evm_client.get_block_number().await?;
    log::debug!("latest block number in evm: {}", end_block);

    let start_block = db_client.get_latest_block_number().await?;
    log::debug!("latest block number stored: {:?}", start_block);

    extractor
        .collect_blocks(start_block.map(|b| b + 1).unwrap_or_default(), end_block)
        .await?;

    Ok(())
}

fn init_logger(log_level: String) -> anyhow::Result<()> {
    let level = log_level
        .parse::<log::LevelFilter>()
        .unwrap_or(log::LevelFilter::Info);

    env_logger::Builder::new().filter(None, level).try_init()?;

    Ok(())
}
