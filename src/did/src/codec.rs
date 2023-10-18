use candid::{CandidType, Decode, Deserialize, Encode};

pub fn encode<T: CandidType>(item: &T) -> Vec<u8> {
    Encode!(item).expect("failed to encode item to candid")
}

pub fn decode<'a, T: CandidType + Deserialize<'a>>(bytes: &'a [u8]) -> T {
    Decode!(bytes, T).expect("failed to decode item from candid")
}

pub fn bincode_encode<T: serde::Serialize>(item: &T) -> Vec<u8> {
    bincode::serialize(item).expect("failed to serialize item with bincode")
}

pub fn bincode_decode<'a, T: serde::Deserialize<'a>>(bytes: &'a [u8]) -> T {
    bincode::deserialize(bytes).expect("failed to deserialize item with bincode")
}

/// A reader for byte data
pub struct ByteChunkReader<'a> {
    position: usize,
    data: &'a [u8],
}

impl<'a> ByteChunkReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, position: 0 }
    }

    /// Reads a chunk of data and update the reader internal pointer
    /// to the beginning of the next chunk.
    /// It panics if not enough data is present
    pub fn read(&mut self, chunk_size: usize) -> &'a [u8] {
        let res = &self.data[self.position..self.position + chunk_size];
        self.position += chunk_size;
        res
    }

    /// Reads a chunk of data and update the reader internal pointer
    /// to the beginning of the next chunk.
    /// It panics if not enough data is present.
    pub fn read_slice<const N: usize>(&mut self) -> &'a [u8; N] {
        self.read(N)
            .try_into()
            .expect("Should read the exact size of bytes")
    }

    /// Reads the remaining data and update the reader internal pointer.
    /// It panics if not enough data is present
    pub fn read_all(&mut self) -> &'a [u8] {
        self.read(self.data.len() - self.position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunck_reader() {
        let data = [0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13];
        let mut reader = ByteChunkReader::new(&data);

        assert_eq!(&[0u8], reader.read(1));
        assert_eq!(&[1u8, 2], reader.read(2));
        assert_eq!(&[3u8, 4u8, 5u8, 6u8], reader.read_slice::<4>());
        assert_eq!(&[7u8], reader.read(1));
        assert_eq!(&[8u8, 9, 10, 11, 12, 13], reader.read_all());
    }
}
