use clap::Parser;
use evm_block_extractor::block_extractor::BlockExtractor;
use evm_block_extractor::storage_clients::gcp_big_query::BigQueryBlockChain;

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

    /// The project ID of the BigQuery table
    #[arg(long = "project-id", short('p'), default_value = "bitfinity-evm")]
    project_id: String,

    /// The dataset ID of the BigQuery table
    /// The dataset ID can be one of the following:
    /// - `testnet`
    /// - `mainnet`
    #[arg(long = "dataset-id", short('d'))]
    dataset_id: String,

    /// Max number of parallel requests in a single RPC batch
    #[arg(long, default_value = "50")]
    max_number_of_requests: usize,

    #[arg(long, default_value = "500")]
    rpc_batch_size: u64,

    /// The service account key in JSON format
    #[arg(long = "sa-key", short('k'), env = "GCP_BLOCK_EXTRACTOR_SA_KEY")]
    sa_key: String,

    /// Log level (default: info, options: trace, debug, info, warn, error)
    #[arg(long, default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logger
    init_logger()?;
    let args = Args::parse();

    // Check if the dataset ID is valid
    if args.dataset_id != "testnet" && args.dataset_id != "mainnet" {
        return Err(anyhow::anyhow!(
            "Invalid dataset ID. The dataset ID can be one of the following: testnet, mainnet"
        ));
    }

    log::info!("{PACKAGE}");
    log::info!("----------------------");
    log::info!("- rpc-url: {}", args.rpc_url);
    log::info!("- project-id: {}", args.project_id);
    log::info!("- dataset-id: {}", args.dataset_id);
    log::info!("- max-number-of-requests: {}", args.max_number_of_requests);
    log::info!("----------------------");

    log::info!("initializing blocks-writer...");

    log::info!("blocks-writer initialized");

    let big_query_client =
        BigQueryBlockChain::new(args.project_id, args.dataset_id, args.sa_key).await?;

    let mut extractor = BlockExtractor::new(
        args.rpc_url,
        args.max_number_of_requests as u64,
        args.rpc_batch_size,
        Box::new(big_query_client.clone()),
    );

    let end_block = extractor.latest_block_number().await?;
    log::debug!("latest block number: {}", end_block);

    let start_block = extractor.latest_block_number_stored().await?;
    log::debug!("latest block number stored: {}", start_block);

    extractor
        .collect_blocks(start_block..=end_block, args.max_number_of_requests)
        .await?;

    Ok(())
}

fn init_logger() -> anyhow::Result<()> {
    env_logger::init();

    Ok(())
}
