mod block_extractor;
mod constants;
mod storage_clients;

use block_extractor::BlockExtractor;
use clap::Parser;

use crate::storage_clients::gcp_bq::BigQueryBlockChain;
use crate::storage_clients::BlockChainDB;

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

    /// The dataset ID of the BigQuery table
    #[arg(long = "dataset-id", short('d'))]
    dataset_id: String,

    /// The table ID of the BigQuery table
    #[arg(long = "table-id", short('t'))]
    table_id: String,

    /// Max number of parallel requests in a single RPC batch
    #[arg(long, default_value = "50")]
    max_number_of_requests: usize,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logger
    init_logger()?;
    let args = Args::parse();

    log::info!("{PACKAGE}");
    log::info!("----------------------");
    log::info!("- rpc-url: {}", args.rpc_url);
    log::info!("- dataset-id: {}", args.dataset_id);
    log::info!("- table-id: {}", args.table_id);
    log::info!("- max-number-of-requests: {}", args.max_number_of_requests);
    log::info!("----------------------");

    log::info!("initializing blocks-writer...");

    log::info!("blocks-writer initialized");

    let big_query_client = BigQueryBlockChain::new(args.dataset_id, args.table_id).await?;

    let mut extractor = BlockExtractor::new(
        args.rpc_url,
        args.max_number_of_requests as u64,
        Box::new(big_query_client.clone()),
    );

    let end_block = extractor.latest_block_number().await.unwrap();

    // get all the blocks on the EVM if you can
    let start_block = end_block - 3600 * 24 * 28;
    let missing_indices = big_query_client
        .get_missing_blocks_in_range(start_block, end_block)
        .await?;

    for chunk in missing_indices.chunks(10000) {
        let chunk = chunk.to_vec();
        extractor
            .collect_blocks(chunk.into_iter(), args.max_number_of_requests)
            .await?;
    }
    extractor
        .collect_blocks(start_block..=end_block, args.max_number_of_requests)
        .await?;

    Ok(())
}

fn init_logger() -> anyhow::Result<()> {
    env_logger::init();

    Ok(())
}
