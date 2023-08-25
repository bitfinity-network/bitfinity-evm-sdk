use std::fs::File;
use std::io::Write;
use std::path::Path;

use ethers_core::types::{Block, Transaction};
use zip::write::{FileOptions, ZipWriter};

pub struct BlocksWriter {
    writer: ZipWriter<File>,
}

impl BlocksWriter {
    /// Try to init a new BlocksWriter
    pub fn new(output_file: &Path) -> anyhow::Result<Self> {
        let file = File::create(output_file)?;

        Ok(Self {
            writer: ZipWriter::new(file),
        })
    }

    /// Put serialized block into archive in a file called `block_{NUMBER}.json`
    pub fn write_block(&mut self, block: &Block<Transaction>) -> anyhow::Result<()> {
        let block_data = serde_json::to_string(block)?;
        self.writer
            .start_file(Self::file_name_from_block(block), FileOptions::default())?;
        self.writer.write_all(block_data.as_bytes())?;

        Ok(())
    }

    fn file_name_from_block(block: &Block<Transaction>) -> String {
        format!("block_0x{:016x}.json", block.number.unwrap().as_u64())
    }
}
