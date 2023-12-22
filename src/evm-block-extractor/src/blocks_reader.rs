use std::fs::File;
use std::path::Path;

use zip::read::ZipArchive;

use crate::constants::{BLOCK_FILE_PREFIX, BLOCK_FILE_SUFFIX};

/// A reader for blocks stored in a zip file
pub struct BlocksReader {
    reader: ZipArchive<File>,
}

impl BlocksReader {
    /// Creates a new reader for the blocks stored in the given zip file
    pub fn new(path: &Path) -> anyhow::Result<Self> {
        let file = File::open(path)?;
        let reader = ZipArchive::new(file)?;

        Ok(Self { reader })
    }

    /// Returns the number of the last block stored in the zip file
    pub fn get_last_block_number(&mut self) -> anyhow::Result<u64> {
        if self.reader.is_empty() {
            anyhow::bail!("No blocks found");
        }
        let last_block_file = self.reader.by_index(self.reader.len() - 1)?;
        let last_block_number = Self::block_number_from_file_name(last_block_file.name())?;

        Ok(last_block_number)
    }

    /// Returns the number of the block by the name of the file
    fn block_number_from_file_name(file_name: &str) -> anyhow::Result<u64> {
        let block_number_str = file_name
            .strip_prefix(BLOCK_FILE_PREFIX)
            .ok_or_else(|| anyhow::anyhow!("Invalid block file name"))?
            .strip_suffix(BLOCK_FILE_SUFFIX)
            .ok_or_else(|| anyhow::anyhow!("Invalid block file name"))?;

        Ok(u64::from_str_radix(block_number_str, 16)?)
    }
}

#[cfg(test)]
mod test {

    use ethers_core::types::Block;
    use tempfile::NamedTempFile;

    use super::*;
    use crate::BlocksWriter;

    #[test]
    fn test_should_get_last_block_number() {
        let file = NamedTempFile::new().unwrap();
        let mut writer = BlocksWriter::new(file.path()).unwrap();

        for number in 0..10 {
            let block = Block {
                number: Some(number.into()),
                ..Default::default()
            };
            writer.write_block(&block).unwrap();
        }
        drop(writer);

        let mut reader = BlocksReader::new(file.path()).unwrap();
        assert_eq!(reader.get_last_block_number().unwrap(), 9);
    }
}
