mod blocks_writer;
mod rpc_client;

use std::path::PathBuf;

use blocks_writer::BlocksWriter;
use clap::Parser;
use ethers_core::types::{Block, BlockNumber, Transaction};
use itertools::Itertools;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const PACKAGE: &str = env!("CARGO_PKG_NAME");

const BLOCKS_PER_REQUEST: usize = rpc_client::MAX_BATCH_REQUESTS; // Max batch size is 5 in EVM

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
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logger
    init_logger()?;
    let args = Args::parse();

    let start_block = args.start_block;
    let end_block = args.end_block.unwrap_or(u64::MAX);

    log::info!("{PACKAGE}");
    log::info!("----------------------");
    log::info!("- rpc-url: {}", args.rpc_url);
    log::info!("- output-file: {}", args.output_file.display());
    log::info!("- start-block: {start_block:#x}");
    log::info!("- end-block: {end_block:#x}");
    log::info!("----------------------");

    log::info!("initializing blocks-writer...");
    let blocks_writer = BlocksWriter::new(&args.output_file)?;
    log::info!("blocks-writer initialized");

    collect_blocks(&args.rpc_url, blocks_writer, start_block, end_block).await?;

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
) -> anyhow::Result<()> {
    for block_numbers in &(start_block..end_block).chunks(BLOCKS_PER_REQUEST) {
        let block_numbers: Vec<BlockNumber> = block_numbers.map(|number| number.into()).collect();
        log::info!(
            "collecting blocks from {} to {}",
            block_numbers.first().unwrap(),
            block_numbers.last().unwrap()
        );
        let blocks = rpc_client::get_blocks_by_number(rpc_url, &block_numbers).await?;
        if blocks.is_empty() {
            log::info!("there are no more blocks available on the EVM");
            break;
        }
        // get tx receipts
        for block in blocks.iter() {
            log::info!(
                "getting receipts for block {}",
                block.number.unwrap().as_u64()
            );
            let receipts = rpc_client::get_receipts_by_number(rpc_url, block).await?;
            log::info!("writing {} receipts", receipts.len());
            blocks_writer.write_receipts(block.number.unwrap().as_u64(), &receipts)?;
        }
        log::info!("writing {} blocks", blocks.len());
        write_blocks(&mut blocks_writer, &blocks)?;

        if blocks.len() < block_numbers.len() {
            log::info!(
                "Found last block to be 0x{:x}",
                blocks.last().unwrap().number.unwrap_or_default()
            );
            break;
        }
    }

    Ok(())
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
