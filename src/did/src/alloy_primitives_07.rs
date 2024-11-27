use crate::{Bytes, H160, H256, H64, U256, U64};

impl From<alloy_primitives_07::Bytes> for Bytes {
    fn from(value: alloy_primitives_07::Bytes) -> Self {
        Bytes(value.0)
    }
}

impl From<Bytes> for alloy_primitives_07::Bytes {
    fn from(value: Bytes) -> Self {
        alloy_primitives_07::Bytes(value.0)
    }
}

impl From<alloy_primitives_07::Address> for H160 {
    fn from(value: alloy_primitives_07::Address) -> Self {
        H160::from_slice(value.as_slice())
    }
}

impl From<H160> for alloy_primitives_07::Address {
    fn from(value: H160) -> Self {
        alloy_primitives_07::Address::from_slice(value.0.as_bytes())
    }
}

impl From<alloy_primitives_07::B64> for H64 {
    fn from(value: alloy_primitives_07::B64) -> Self {
        H64::from_slice(value.as_slice())
    }
}

impl From<H64> for alloy_primitives_07::B64 {
    fn from(value: H64) -> Self {
        alloy_primitives_07::B64::from_slice(value.0.as_bytes())
    }
}

impl From<alloy_primitives_07::B256> for H256 {
    fn from(value: alloy_primitives_07::B256) -> Self {
        H256::from_slice(value.as_slice())
    }
}

impl From<H256> for alloy_primitives_07::B256 {
    fn from(value: H256) -> Self {
        alloy_primitives_07::B256::from_slice(value.0.as_bytes())
    }
}

impl From<alloy_primitives_07::U256> for U256 {
    fn from(value: alloy_primitives_07::U256) -> Self {
        U256::from_little_endian(value.as_le_slice())
    }
}

impl From<U256> for alloy_primitives_07::U256 {
    fn from(value: U256) -> Self {
        let mut bytes = [0u8; U256::BYTE_SIZE];
        value.0.to_little_endian(&mut bytes);
        alloy_primitives_07::U256::from_le_bytes(bytes)
    }
}

impl From<alloy_primitives_07::U64> for U64 {
    fn from(value: alloy_primitives_07::U64) -> Self {
        U64::from_little_endian(value.as_le_slice())
    }
}

impl From<U64> for alloy_primitives_07::U64 {
    fn from(value: U64) -> Self {
        let mut bytes = [0u8; U64::BYTE_SIZE];
        value.0.to_little_endian(&mut bytes);
        alloy_primitives_07::U64::from_le_bytes(bytes)
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn test_alloy_bytes_roundtrip() {
        let value = Bytes(bytes::Bytes::from(vec![
            rand::random::<u8>(),
            rand::random::<u8>(),
            rand::random::<u8>(),
        ]));

        let alloy_bytes = alloy_primitives_07::Bytes::from(value.clone());
        let decoded_value = Bytes::from(alloy_bytes);

        assert_eq!(value, decoded_value);
    }

    #[test]
    fn test_alloy_address_roundtrip() {
        let value: H160 = ethereum_types::H160::random().into();

        let alloy_address = alloy_primitives_07::Address::from(value.clone());
        let decoded_value = H160::from(alloy_address);

        assert_eq!(value, decoded_value);
    }

    #[test]
    fn test_alloy_h256_roundtrip() {
        let value: H256 = ethereum_types::H256::random().into();

        let alloy_h256 = alloy_primitives_07::B256::from(value.clone());
        let decoded_value = H256::from(alloy_h256);

        assert_eq!(value, decoded_value);
    }

    #[test]
    fn test_alloy_h64_roundtrip() {
        let value: H64 = ethereum_types::H64::random().into();

        let alloy_h64 = alloy_primitives_07::B64::from(value.clone());
        let decoded_value = H64::from(alloy_h64);

        assert_eq!(value, decoded_value);
    }

    #[test]
    fn test_alloy_u256_roundtrip() {
        let value: U256 = ethereum_types::U256::from(rand::random::<u128>()).into();

        let alloy_u256: alloy_primitives_07::U256 = value.clone().into();
        let decoded_value: U256 = alloy_u256.into();

        assert_eq!(value, decoded_value);
    }

    #[test]
    fn test_alloy_u64_roundtrip() {
        let value: U64 = ethereum_types::U64::from(rand::random::<u64>()).into();

        let alloy_u64: alloy_primitives_07::U64 = value.into();
        let decoded_value: U64 = alloy_u64.into();

        assert_eq!(value, decoded_value);
    }
}
