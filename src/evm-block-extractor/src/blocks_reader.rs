use std::fs::File;
use std::path::Path;

use zip::read::ZipArchive;

pub struct BlocksReader {
    reader: ZipArchive<File>,
}

impl BlocksReader {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        let file = File::open(path)?;
        let reader = ZipArchive::new(file)?;

        Ok(Self { reader })
    }

    pub fn get_last_block_number(&mut self) -> anyhow::Result<u64> {
        if self.reader.len() == 0 {
            anyhow::bail!("No blocks found");
        }
        let last_block_file = self.reader.by_index(self.reader.len() - 1)?;
        let last_block_number = Self::block_number_from_file_name(last_block_file.name())?;

        Ok(last_block_number)
    }

    fn block_number_from_file_name(file_name: &str) -> anyhow::Result<u64> {
        let prefix = if file_name.starts_with("block") {
            "block_0x"
        } else {
            "receipt_0x"
        };

        let block_number_str = file_name
            .strip_prefix(prefix)
            .ok_or_else(|| anyhow::anyhow!("Invalid block file name"))?
            .strip_suffix(".json")
            .ok_or_else(|| anyhow::anyhow!("Invalid block file name"))?;

        Ok(u64::from_str_radix(block_number_str, 16)?)
    }
}

#[cfg(test)]
mod test {

    use super::*;

    use crate::BlocksWriter;

    use ethers_core::types::Block;
    use tempfile::NamedTempFile;

    #[test]
    fn test_should_get_last_block_number() {
        let file = NamedTempFile::new().unwrap();
        let mut writer = BlocksWriter::open(file.path(), false).unwrap();

        for number in 0..10 {
            let block = Block {
                number: Some(number.into()),
                ..Default::default()
            };
            writer.write_block(&block).unwrap();
        }
        drop(writer);

        let mut reader = BlocksReader::open(file.path()).unwrap();
        assert_eq!(reader.get_last_block_number().unwrap(), 9);
    }
}
