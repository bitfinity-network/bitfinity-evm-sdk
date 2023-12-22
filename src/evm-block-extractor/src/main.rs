mod blocks_reader;
mod blocks_writer;
mod constants;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::sleep;
use std::time::{Duration, Instant};

use blocks_reader::BlocksReader;
use blocks_writer::BlocksWriter;
use chrono::Local;
use clap::Parser;
use ethereum_json_rpc_client::reqwest::ReqwestClient;
use ethereum_json_rpc_client::EthJsonRcpClient;
use ethers_core::types::{Block, BlockNumber, Transaction};
use itertools::Itertools;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const PACKAGE: &str = env!("CARGO_PKG_NAME");

/// The rpc client splits batches into chunks itself, so here we just specify the number of blocks to hold in memory
const DEFAULT_MAX_BATCH_SIZE: usize = 500;

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

    /// Output directory to write archives block to
    #[arg(long = "output-dir", short('o'))]
    output_dir: PathBuf,

    /// block to start with
    #[arg(long, short('s'), default_value = "0")]
    start_block: u64,

    /// block to start with (if not provided, all blocks will be loaded)
    #[arg(long, short('e'))]
    end_block: Option<u64>,

    /// Max number of requests in a single RPC batch
    #[arg(long)]
    batch_size: Option<usize>,

    /// Interval in seconds to fetch new blocks
    #[arg(long, short('i'), default_value = "60")]
    fetch_interval: u64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logger
    init_logger()?;
    let args = Args::parse();

    // create output dir if it doesn't exist
    if !args.output_dir.exists() {
        std::fs::create_dir_all(&args.output_dir)?;
    } else if args.output_dir.is_file() {
        anyhow::bail!("{}: must be a directory", args.output_dir.display());
    }

    let start_block = get_last_block_number_from_output_dir(&args.output_dir)
        .map(|last_block_number| last_block_number + 1)
        .unwrap_or(args.start_block);
    let end_block = args.end_block.unwrap_or(u64::MAX);
    let fetch_interval = Duration::from_secs(args.fetch_interval);
    let max_batch_size = args.batch_size.unwrap_or(DEFAULT_MAX_BATCH_SIZE);

    log::info!("{PACKAGE}");
    log::info!("----------------------");
    log::info!("- rpc-url: {}", args.rpc_url);
    log::info!("- output-dir: {}", args.output_dir.display());
    log::info!("- start-block: {start_block:#x}");
    log::info!("- end-block: {end_block:#x}");
    log::info!("- fetch-interval: {}", fetch_interval.as_secs());
    log::info!("- max-batch-size: {max_batch_size}");
    log::info!("----------------------");

    // setup signal handler
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })?;

    let mut next_loop_start_block = start_block;
    'main_loop: loop {
        let output_file = get_output_file(&args.output_dir);
        log::info!("initializing blocks-writer...");
        let mut blocks_writer = BlocksWriter::new(&output_file)?;
        log::info!("blocks-writer initialized");

        next_loop_start_block = match collect_blocks(
            &args.rpc_url,
            &mut blocks_writer,
            next_loop_start_block,
            end_block,
            max_batch_size,
        )
        .await
        {
            Ok(last_block_fetched) => last_block_fetched + 1,
            Err((last_block_fetched, err)) => {
                log::error!("error collecting blocks: {}", err);
                // remove output file
                if last_block_fetched == start_block {
                    std::fs::remove_file(&output_file)?;
                    last_block_fetched
                } else {
                    last_block_fetched + 1
                }
            }
        };

        if next_loop_start_block >= end_block {
            log::info!("reached end block, exiting...");
            break 'main_loop;
        }

        // sleep for provided interval
        let now = Instant::now();
        while now.elapsed() < fetch_interval {
            sleep(Duration::from_secs(1));
            if !running.load(Ordering::SeqCst) {
                log::info!("received SIGINT, exiting...");
                break 'main_loop;
            }
        }
    }

    Ok(())
}

fn init_logger() -> anyhow::Result<()> {
    env_logger::init();

    Ok(())
}

async fn collect_blocks(
    rpc_url: &str,
    blocks_writer: &mut BlocksWriter,
    start_block: u64,
    end_block: u64,
    max_batch_size: usize,
) -> Result<u64, (u64, anyhow::Error)> {
    let client = EthJsonRcpClient::new(ReqwestClient::new(rpc_url.to_string()));

    let mut last_block_fetched = start_block;
    for block_numbers in &(start_block..end_block).chunks(max_batch_size) {
        let block_numbers: Vec<BlockNumber> = block_numbers.map(|number| number.into()).collect();
        log::info!(
            "collecting blocks from {} to {}",
            block_numbers.first().unwrap(),
            block_numbers.last().unwrap()
        );
        let blocks = client
            .get_full_blocks_by_number(block_numbers.clone(), max_batch_size)
            .await
            .map_err(|e| (last_block_fetched, e))?;
        if blocks.is_empty() {
            log::info!("there are no more blocks available on the EVM");
            break;
        }
        // get tx receipts
        for block in blocks.iter() {
            log::info!(
                "getting {} receipts for block {}",
                block.transactions.len(),
                block.number.unwrap().as_u64()
            );
            let tx_hashes = block.transactions.iter().map(|tx| tx.hash());
            let receipts = client
                .get_receipts_by_hash(tx_hashes, max_batch_size)
                .await
                .map_err(|e| (last_block_fetched, e))?;
            log::info!("writing {} receipts", receipts.len());
            blocks_writer
                .write_receipts(block.number.unwrap().as_u64(), &receipts)
                .map_err(|e| (last_block_fetched, e))?;
        }
        log::info!("writing {} blocks", blocks.len());
        write_blocks(blocks_writer, &blocks).map_err(|e| (last_block_fetched, e))?;

        if blocks.len() < block_numbers.len() {
            log::info!(
                "Found last block to be 0x{:x}",
                blocks.last().unwrap().number.unwrap_or_default()
            );
            break;
        }
        last_block_fetched = blocks.last().unwrap().number.unwrap().as_u64();
    }

    Ok(last_block_fetched)
}

fn write_blocks(
    blocks_writer: &mut BlocksWriter,
    blocks: &[Block<Transaction>],
) -> anyhow::Result<()> {
    for block in blocks {
        blocks_writer.write_block(block)?;
    }

    Ok(())
}

/// Scans the output dir for existing block files and returns the last block number
fn get_last_block_number_from_output_dir(output_dir: &Path) -> Option<u64> {
    // scan files in output dir
    let mut last_block: Option<u64> = None;
    let mut output_dir = std::fs::read_dir(output_dir).ok()?;
    for file in output_dir.by_ref() {
        let file = file.ok()?;
        log::info!("found file: {}", file.path().display());
        last_block = match (
            last_block,
            get_last_block_number_from_output_file(&file.path()),
        ) {
            (last_block, None) => last_block,
            (Some(last_block), Some(file_last_block)) => Some(last_block.min(file_last_block)),
            (None, Some(file_last_block)) => Some(file_last_block),
        };
    }

    last_block
}

/// Returns the last block number from the given output file
fn get_last_block_number_from_output_file(output_file: &Path) -> Option<u64> {
    let mut reader = BlocksReader::new(Path::new(output_file)).ok()?;
    reader.get_last_block_number().ok()
}

/// Returns the output file path for the given output dir based on the current time
fn get_output_file(output_dir: &Path) -> PathBuf {
    let now = Local::now().format("%Y%m%d%H");
    let mut index = 0;
    loop {
        let mut output_file = output_dir.to_path_buf();
        output_file.push(format!("blocks_{now}_{index}.zip"));

        if output_file.exists() {
            index += 1;
            continue;
        } else {
            return output_file;
        }
    }
}
