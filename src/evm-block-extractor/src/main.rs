mod blocks_reader;
mod blocks_writer;
mod constants;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use blocks_reader::BlocksReader;
use blocks_writer::BlocksWriter;
use clap::Parser;
use ethereum_json_rpc_client::reqwest::ReqwestClient;
use ethereum_json_rpc_client::EthJsonRcpClient;
use ethers_core::types::{Block, BlockNumber, Transaction};

use tokio::sync::{mpsc, Semaphore};
use tokio::time::{self, Instant};

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

async fn collect_blocks(
    rpc_url: &str,
    mut blocks_writer: BlocksWriter,
    start_block: u64,
    end_block: u64,
    max_no_of_requests: usize,
    total_time: u64,
) -> anyhow::Result<()> {
    let client = EthJsonRcpClient::new(ReqwestClient::new(rpc_url.to_string()));

    let mut total_blocks = 0;

    let semaphore = Arc::new(Semaphore::new(max_no_of_requests));

    let (tx, mut rx) = mpsc::unbounded_channel::<Block<Transaction>>();

    let mut tasks = Vec::new();

    let mut interval = time::interval(Duration::from_secs(1));
    let start = Instant::now();

    for block_number in start_block..=end_block {
        let permit = semaphore.clone().acquire_owned().await?;

        let block_number = BlockNumber::Number(block_number.into());

        let tx = tx.clone();
        let client = client.clone();

        let task = tokio::spawn(async move {
            let block = client.get_full_block_by_number(block_number).await;
            drop(permit);

            match block {
                Ok(block) => {
                    tx.send(block).expect("failed to send block");
                }
                Err(err) => {
                    log::error!("error getting block: {}", err);
                }
            }
        });

        tasks.push(task);

        if start.elapsed().as_secs() >= total_time {
            break;
        }

        interval.tick().await;
    }

    drop(tx);

    while let Some(block) = rx.recv().await {
        log::info!(
            "getting {} receipts for block {}",
            block.transactions.len(),
            block.number.unwrap().as_u64()
        );
        let tx_hashes = block.transactions.iter().map(|tx| tx.hash());
        let receipts = client
            .get_receipts_by_hash(tx_hashes, max_no_of_requests)
            .await?;

        log::info!("writing {} receipts", receipts.len());
        blocks_writer.write_receipts(block.number.unwrap().as_u64(), &receipts)?;

        log::info!("writing block {}", block.number.unwrap().as_u64());

        blocks_writer.write_block(&block)?;
        total_blocks += 1;
    }

    for task in tasks {
        task.await?;
    }

    log::info!("total blocks: {}", total_blocks);

    Ok(())
}

fn get_last_block_number_from_output_file(output_file: &Path) -> Option<u64> {
    let mut reader = BlocksReader::new(Path::new(output_file)).ok()?;
    reader.get_last_block_number().ok()
}
