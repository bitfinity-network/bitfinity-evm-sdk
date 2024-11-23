use std::fmt;
use std::rc::Rc;

use candid::types::*;
use candid::CandidType;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct Bytes(pub alloy::primitives::Bytes);

impl Bytes {
    pub fn from_hex_str(mut s: &str) -> Result<Self, hex::FromHexError> {
        if s.starts_with("0x") || s.starts_with("0X") {
            s = &s[2..]
        }
        let bytes = hex::decode(s)?;
        Ok(Self(alloy::primitives::Bytes::from(bytes)))
    }

    pub fn to_hex_str(&self) -> String {
        format!("0x{self:x}")
    }
}

impl alloy::rlp::Encodable for Bytes {
    fn encode(&self, out: &mut dyn bytes::BufMut) {
        self.0.encode(out);
    }
}

impl alloy::rlp::Decodable for Bytes {
    fn decode(buf: &mut &[u8]) -> alloy::rlp::Result<Self> {
        Ok(Self(alloy::primitives::Bytes::decode(buf)?))
    }
}

impl From<Bytes> for bytes::Bytes {
    fn from(value: Bytes) -> Self {
        value.0.0
    }
}

impl From<bytes::Bytes> for Bytes {
    fn from(value: bytes::Bytes) -> Self {
        Bytes(value.into())
    }
}

impl From<Vec<u8>> for Bytes {
    fn from(value: Vec<u8>) -> Self {
        Bytes(value.into())
    }
}

impl From<Bytes> for Vec<u8> {
    fn from(value: Bytes) -> Self {
        value.0.into()
    }
}

impl From<Bytes> for alloy::primitives::Bytes {
    fn from(value: Bytes) -> Self {
        value.0
    }
}

impl From<alloy::primitives::Bytes> for Bytes {
    fn from(value: alloy::primitives::Bytes) -> Self {
        Bytes(value)
    }
}

impl fmt::LowerHex for Bytes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl CandidType for Bytes {
    fn _ty() -> candid::types::Type {
        Type(Rc::new(TypeInner::Text))
    }

    fn idl_serialize<S>(&self, serializer: S) -> Result<(), S::Error>
    where
        S: candid::types::Serializer,
    {
        serializer.serialize_text(&self.to_hex_str())
    }
}

impl Serialize for Bytes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_hex_str())
    }
}

impl<'de> Deserialize<'de> for Bytes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Bytes::from_hex_str(&value).map_err(|e| serde::de::Error::custom(e.to_string()))
    }
}

#[cfg(test)]
mod tests {

    use candid::{Decode, Encode};

    use super::*;

    #[test]
    fn test_candid_type_bytes() {
        let value = Bytes(bytes::Bytes::from(vec![
            rand::random::<u8>(),
            rand::random::<u8>(),
            rand::random::<u8>(),
        ]));

        let encoded = Encode!(&value).unwrap();
        let decoded = Decode!(&encoded, Bytes).unwrap();

        assert_eq!(value, decoded);
    }

    #[test]
    fn test_bytes_fmt_lower_hex() {
        let value = Bytes(bytes::Bytes::from(vec![
            rand::random::<u8>(),
            rand::random::<u8>(),
            rand::random::<u8>(),
        ]));
        let lower_hex = value.to_hex_str();
        assert!(lower_hex.starts_with("0x"));
        assert_eq!(value, Bytes::from_hex_str(&lower_hex).unwrap());
    }

    #[test]
    fn test_bytes_serde_serialization() {
        let value = Bytes(bytes::Bytes::from(vec![
            rand::random::<u8>(),
            rand::random::<u8>(),
            rand::random::<u8>(),
        ]));

        let encoded_value = serde_json::json!(&value);
        let decoded_value: Bytes = serde_json::from_value(encoded_value).unwrap();

        assert_eq!(value, decoded_value);
    }
}
