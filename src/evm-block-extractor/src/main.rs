

mod constants;
mod block_extractor;
mod storage_clients;
/* 
use clap::Parser;

use block_extractor::BlockExtractor;


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

    /// Output ZIP file to write blocks to
    #[arg(long = "output", short('o'))]
    output_file: PathBuf,

    /// block to start with
    #[arg(long, short('s'), default_value = "0")]
    start_block: u64,

    /// block to start with (if not provided, all blocks will be loaded)
    #[arg(long, short('e'))]
    end_block: Option<u64>,

    /// Max number of parallel requests in a single RPC batch
    #[arg(long, default_value = "50")]
    max_number_of_requests: usize,

    /// Total time to send concurrent requests
    #[arg(long, default_value = "10")]
    total_time: u64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logger
    init_logger()?;
    let args = Args::parse();

    let (start_block, append) = match get_last_block_number_from_output_file(&args.output_file) {
        Some(last_block_number) => {
            log::info!(
                "last block number found in output file: {}",
                last_block_number
            );
            (last_block_number + 1, true)
        }
        None => (args.start_block, false),
    };
    let end_block = args.end_block.unwrap_or(u64::MAX);

    log::info!("{PACKAGE}");
    log::info!("----------------------");
    log::info!("- rpc-url: {}", args.rpc_url);
    log::info!("- output-file: {}", args.output_file.display());
    log::info!("- start-block: {start_block:#x}");
    log::info!("- end-block: {end_block:#x}");
    log::info!("- max-number-of-requests: {}", args.max_number_of_requests);
    log::info!("- total-time: {}", args.total_time);
    log::info!("----------------------");

    log::info!("initializing blocks-writer...");
    let blocks_writer = BlocksWriter::new(&args.output_file, append)?;
    log::info!("blocks-writer initialized");

    collect_blocks(
        &args.rpc_url,
        blocks_writer,
        start_block,
        end_block,
        args.max_number_of_requests,
        args.total_time,
    )
    .await?;

    Ok(())
}

fn init_logger() -> anyhow::Result<()> {
    env_logger::init();

    Ok(())
}
*/

fn main() {}