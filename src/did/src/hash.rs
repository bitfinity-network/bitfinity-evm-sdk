use std::borrow::Cow;
use std::fmt;
use std::rc::Rc;
use std::str::FromStr;

use alloy::hex::FromHexError;
use candid::types::{Type, TypeInner};
use candid::CandidType;
use derive_more::Display;
use ic_stable_structures::{Bound, Bounded, Storable};

#[derive(Debug, Default, Clone, PartialOrd, Ord, Eq, PartialEq, Display, Hash)]
pub struct Hash<T>(pub T);

///Fixed-size uninterpreted hash type with 8 bytes (64 bits) size.
pub type H64 = Hash<alloy::primitives::B64>;
///Fixed-size uninterpreted hash type with 20 bytes (160 bits) size.
pub type H160 = Hash<alloy::primitives::Address>;
///Fixed-size uninterpreted hash type with 32 bytes (256 bits) size.
pub type H256 = Hash<alloy::primitives::B256>;

pub fn from_hex_str<const SIZE: usize>(mut s: &str) -> Result<[u8; SIZE], FromHexError> {
    if s.starts_with("0x") || s.starts_with("0X") {
        s = &s[2..];
    }

    let mut result = [0u8; SIZE];
    alloy::hex::decode_to_slice(s, &mut result).and(Ok(result))
}

impl serde::Serialize for H64 {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_hex_str())
    }
}

impl serde::Serialize for H160 {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_hex_str())
    }
}

impl serde::Serialize for H256 {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_hex_str())
    }
}

impl<'de> serde::Deserialize<'de> for H64 {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        H64::from_hex_str(&s).map_err(serde::de::Error::custom)
    }
}

impl<'de> serde::Deserialize<'de> for H160 {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        H160::from_hex_str(&s).map_err(serde::de::Error::custom)
    }
}

impl<'de> serde::Deserialize<'de> for H256 {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        H256::from_hex_str(&s).map_err(serde::de::Error::custom)
    }
}

impl H64 {
    pub const BYTE_SIZE: usize = 8;

    pub fn new(value: alloy::primitives::B64) -> Self {
        Self(value)
    }

    pub fn from_slice(slice: &[u8]) -> Self {
        Self(alloy::primitives::B64::from_slice(slice))
    }

    pub fn from_hex_str(s: &str) -> Result<Self, FromHexError> {
        Ok(Self(alloy::primitives::B64::from(from_hex_str::<8>(s)?)))
    }

    pub fn to_hex_str(&self) -> String {
        format!("0x{self:x}")
    }

    pub const fn zero() -> Self {
        Self(alloy::primitives::B64::ZERO)
    }
}

impl FromStr for H64 {
    type Err = FromHexError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_hex_str(s)
    }
}

impl H160 {
    pub const BYTE_SIZE: usize = 20;

    pub fn new(value: alloy::primitives::Address) -> Self {
        Self(value)
    }

    pub fn from_slice(slice: &[u8]) -> Self {
        Self(alloy::primitives::Address::from_slice(slice))
    }

    pub fn from_hex_str(s: &str) -> Result<Self, FromHexError> {
        Ok(Self(alloy::primitives::Address::from(from_hex_str::<20>(
            s,
        )?)))
    }

    pub fn to_hex_str(&self) -> String {
        format!("0x{self:x}")
    }

    pub const fn zero() -> Self {
        Self(alloy::primitives::Address::ZERO)
    }
}

impl FromStr for H160 {
    type Err = FromHexError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_hex_str(s)
    }
}

impl H256 {
    pub const BYTE_SIZE: usize = 32;

    pub fn new(value: alloy::primitives::B256) -> Self {
        Self(value)
    }

    pub fn from_slice(slice: &[u8]) -> Self {
        Self(alloy::primitives::B256::from_slice(slice))
    }

    pub fn from_hex_str(s: &str) -> Result<Self, FromHexError> {
        Ok(Self(alloy::primitives::B256::from(from_hex_str::<32>(s)?)))
    }

    pub fn to_hex_str(&self) -> String {
        format!("0x{self:x}")
    }

    pub const fn zero() -> Self {
        Self(alloy::primitives::B256::ZERO)
    }
}

impl FromStr for H256 {
    type Err = FromHexError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_hex_str(s)
    }
}

impl Storable for H64 {
    fn to_bytes(&self) -> Cow<'_, [u8]> {
        self.0.as_slice().into()
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Self(alloy::primitives::B64::from_slice(bytes.as_ref()))
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: H64::BYTE_SIZE as u32,
        is_fixed_size: true,
    };
}

impl Storable for H160 {
    fn to_bytes(&self) -> Cow<'_, [u8]> {
        self.0.as_slice().into()
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Self(alloy::primitives::Address::from_slice(bytes.as_ref()))
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: H160::BYTE_SIZE as u32,
        is_fixed_size: true,
    };
}

impl Storable for H256 {
    fn to_bytes(&self) -> Cow<'_, [u8]> {
        self.0.as_slice().into()
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Self(alloy::primitives::B256::from_slice(bytes.as_ref()))
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: H256::BYTE_SIZE as u32,
        is_fixed_size: true,
    };
}

impl CandidType for H64 {
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

impl CandidType for H160 {
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

impl CandidType for H256 {
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

impl alloy::rlp::Encodable for H64 {
    fn encode(&self, out: &mut dyn bytes::BufMut) {
        self.0.encode(out);
    }
}

impl alloy::rlp::Decodable for H64 {
    fn decode(buf: &mut &[u8]) -> alloy::rlp::Result<Self> {
        Ok(Self(alloy::primitives::B64::decode(buf)?))
    }
}

impl alloy::rlp::Encodable for H160 {
    fn encode(&self, out: &mut dyn bytes::BufMut) {
        self.0.encode(out);
    }
}

impl alloy::rlp::Decodable for H160 {
    fn decode(buf: &mut &[u8]) -> alloy::rlp::Result<Self> {
        Ok(Self(alloy::primitives::Address::decode(buf)?))
    }
}

impl alloy::rlp::Encodable for H256 {
    fn encode(&self, out: &mut dyn bytes::BufMut) {
        self.0.encode(out);
    }
}

impl alloy::rlp::Decodable for H256 {
    fn decode(buf: &mut &[u8]) -> alloy::rlp::Result<Self> {
        Ok(Self(alloy::primitives::B256::decode(buf)?))
    }
}

impl Bounded for H64 {
    const MIN: H64 = Hash::<alloy::primitives::B64>(alloy::primitives::B64::new([u8::MIN; 8]));
    const MAX: H64 = Hash::<alloy::primitives::B64>(alloy::primitives::B64::new([u8::MAX; 8]));
}

impl Bounded for H160 {
    const MIN: H160 =
        Hash::<alloy::primitives::Address>(alloy::primitives::Address::new([u8::MIN; 20]));
    const MAX: H160 =
        Hash::<alloy::primitives::Address>(alloy::primitives::Address::new([u8::MAX; 20]));
}

impl Bounded for H256 {
    const MIN: H256 = Hash::<alloy::primitives::B256>(alloy::primitives::B256::new([u8::MIN; 32]));
    const MAX: H256 = Hash::<alloy::primitives::B256>(alloy::primitives::B256::new([u8::MAX; 32]));
}

impl From<H64> for alloy::primitives::B64 {
    fn from(value: H64) -> Self {
        value.0
    }
}

impl From<H160> for alloy::primitives::Address {
    fn from(value: H160) -> Self {
        value.0
    }
}

impl From<H256> for alloy::primitives::B256 {
    fn from(value: H256) -> Self {
        value.0
    }
}

impl From<alloy::primitives::B64> for H64 {
    fn from(value: alloy::primitives::B64) -> Self {
        Hash(value)
    }
}

impl From<alloy::primitives::Address> for H160 {
    fn from(value: alloy::primitives::Address) -> Self {
        Hash(value)
    }
}

impl From<alloy::primitives::B256> for H256 {
    fn from(value: alloy::primitives::B256) -> Self {
        Hash(value)
    }
}

impl From<[u8; 8]> for H64 {
    fn from(value: [u8; 8]) -> Self {
        Hash(value.into())
    }
}

impl From<[u8; 20]> for H160 {
    fn from(value: [u8; 20]) -> Self {
        Hash(value.into())
    }
}

impl From<[u8; 32]> for H256 {
    fn from(value: [u8; 32]) -> Self {
        Hash(value.into())
    }
}

impl fmt::LowerHex for H64 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::LowerHex::fmt(&self.0, f)
    }
}

impl fmt::LowerHex for H160 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::LowerHex::fmt(&self.0, f)
    }
}

impl fmt::LowerHex for H256 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::LowerHex::fmt(&self.0, f)
    }
}

#[cfg(test)]
mod tests {
    use candid::{Decode, Encode};
    use ic_stable_structures::Storable;

    use super::*;

    fn generate_hex_str(size: usize) -> (Vec<u8>, String) {
        (0..size)
            .map(|i| {
                let val = (i * 13 % 255) as u8;
                (val, format!("{val:02x}"))
            })
            .unzip()
    }

    #[test]
    fn test_storable_h64() {
        let bytes: Vec<_> = (0..(H64::BYTE_SIZE as u8)).collect();
        let h64 = H64::from_slice(&bytes);

        let serialized = h64.to_bytes();
        let deserialized = H64::from_bytes(serialized);

        assert_eq!(h64, deserialized);
    }

    #[test]
    fn test_storable_h160() {
        let bytes: Vec<_> = (0..(H160::BYTE_SIZE as u8)).collect();
        let h160 = H160::from_slice(&bytes);

        let serialized = h160.to_bytes();
        let deserialized = H160::from_bytes(serialized);

        assert_eq!(h160, deserialized);
    }

    #[test]
    fn test_storable_h256() {
        let bytes: Vec<_> = (0..(H256::BYTE_SIZE as u8)).collect();
        let h256 = H256::from_slice(&bytes);

        let serialized = h256.to_bytes();
        let deserialized = H256::from_bytes(serialized);

        assert_eq!(h256, deserialized);
    }

    #[test]
    fn test_h64_from_str() {
        let (hex_val, str_val) = generate_hex_str(8);
        println!("str_val: {}", str_val);
        let value = H64::from_slice(&hex_val);
        assert_eq!(value, H64::from_hex_str(&str_val).unwrap());

        assert_eq!(value, H64::from_hex_str(&format!("0x{str_val}")).unwrap());
        assert_eq!(value, H64::from_hex_str(&format!("0X{str_val}")).unwrap());
        assert_eq!(
            value,
            H64::from_hex_str(&format!("0x{}", str_val.to_uppercase())).unwrap()
        );

        assert!(H64::from_hex_str("").is_err());
        assert!(H64::from_hex_str("01").is_err());
        assert!(H64::from_hex_str("012").is_err());
        assert!(H64::from_hex_str(&str_val.replace('0', "g")).is_err());
    }

    #[test]
    fn test_h160_from_str() {
        let (hex_val, str_val) = generate_hex_str(20);
        let value = H160::from_slice(&hex_val);
        assert_eq!(value, H160::from_hex_str(&str_val).unwrap());

        assert_eq!(value, H160::from_hex_str(&format!("0x{str_val}")).unwrap());
        assert_eq!(value, H160::from_hex_str(&format!("0X{str_val}")).unwrap());
        assert_eq!(
            value,
            H160::from_hex_str(&format!("0x{}", str_val.to_uppercase())).unwrap()
        );

        assert!(H160::from_hex_str("").is_err());
        assert!(H160::from_hex_str("01").is_err());
        assert!(H160::from_hex_str("012").is_err());
        assert!(H160::from_hex_str(&str_val.replace('0', "g")).is_err());
    }

    #[test]
    fn test_h256_from_str() {
        let (hex_val, str_val) = generate_hex_str(32);
        let value = H256::from_slice(&hex_val);
        assert_eq!(value, H256::from_hex_str(&str_val).unwrap());

        assert_eq!(value, H256::from_hex_str(&format!("0x{str_val}")).unwrap());
        assert_eq!(value, H256::from_hex_str(&format!("0X{str_val}")).unwrap());
        assert_eq!(
            value,
            H256::from_hex_str(&format!("0x{}", str_val.to_uppercase())).unwrap()
        );

        assert!(H256::from_hex_str("").is_err());
        assert!(H256::from_hex_str("01").is_err());
        assert!(H256::from_hex_str("012").is_err());
        assert!(H256::from_hex_str(&str_val.replace('0', "g")).is_err());
    }

    #[test]
    fn test_hex_from_str_returns_error() {
        let (_, str_val) = generate_hex_str(31);
        let val = H256::from_hex_str(&str_val);
        assert_eq!(val.unwrap_err(), FromHexError::InvalidStringLength);

        let (_, str_val) = generate_hex_str(50);
        let val = H256::from_hex_str(&str_val);
        assert_eq!(val.unwrap_err(), FromHexError::InvalidStringLength);
    }

    #[test]
    fn test_rlp_encoding_decoding_h256() {
        let (hex_val, _) = generate_hex_str(32);
        let value = H256::from_slice(&hex_val);
        let encoded = alloy::rlp::encode(&value);

        let decoded = alloy::rlp::decode_exact::<H256>(&encoded).unwrap();
        assert_eq!(value, decoded);
    }

    #[test]
    fn test_rlp_encoding_decoding_h160() {
        let (hex_val, _) = generate_hex_str(20);
        let value = H160::from_slice(&hex_val);
        let encoded = alloy::rlp::encode(&value);

        let decoded = alloy::rlp::decode_exact::<H160>(&encoded).unwrap();
        assert_eq!(value, decoded);
    }

    #[test]
    fn test_rlp_encoding_decoding_h64() {
        let (hex_val, _) = generate_hex_str(8);
        let value = H64::from_slice(&hex_val);
        let encoded = alloy::rlp::encode(&value);

        let decoded = alloy::rlp::decode_exact::<H64>(&encoded).unwrap();
        assert_eq!(value, decoded);
    }

    #[test]
    fn test_candid_type_h64() {
        let bytes: Vec<_> = (0..8).collect();
        let h64 = H64::from_slice(&bytes);

        let encoded = Encode!(&h64).unwrap();
        let decoded = Decode!(&encoded, H64).unwrap();

        assert_eq!(h64, decoded);
    }

    #[test]
    fn test_candid_type_h160() {
        let bytes: Vec<_> = (0..20).collect();
        let h160 = H160::from_slice(&bytes);

        let encoded = Encode!(&h160).unwrap();
        let decoded = Decode!(&encoded, H160).unwrap();

        assert_eq!(h160, decoded);
    }

    #[test]
    fn test_candid_type_h256() {
        let bytes: Vec<_> = (0..32).collect();
        let h256 = H256::from_slice(&bytes);

        let encoded = Encode!(&h256).unwrap();
        let decoded = Decode!(&encoded, H256).unwrap();

        assert_eq!(h256, decoded);
    }

    #[test]
    fn test_serde_h64() {
        let h64 = H64::new(alloy::primitives::B64::random());

        let encoded = serde_json::json!(&h64);
        let decoded = serde_json::from_value(encoded).unwrap();

        assert_eq!(h64, decoded);
    }

    #[test]
    fn test_serde_h160() {
        let h160 = H160::new(alloy::primitives::Address::random());

        let encoded = serde_json::json!(&h160);
        let decoded = serde_json::from_value(encoded).unwrap();

        assert_eq!(h160, decoded);
    }

    #[test]
    fn test_serde_h256() {
        let h256 = H256::new(alloy::primitives::B256::random());

        let encoded = serde_json::json!(&h256);
        let decoded = serde_json::from_value(encoded).unwrap();

        assert_eq!(h256, decoded);
    }

    #[test]
    fn test_h64_fmt_lower_hex() {
        let value: H64 = alloy::primitives::B64::random().into();
        let lower_hex = value.to_hex_str();
        assert!(lower_hex.starts_with("0x"));
        assert_eq!(value, H64::from_hex_str(&lower_hex).unwrap());
    }

    #[test]
    fn test_h160_fmt_lower_hex() {
        let value: H160 = alloy::primitives::Address::random().into();
        let lower_hex = value.to_hex_str();
        assert!(lower_hex.starts_with("0x"));
        assert_eq!(value, H160::from_hex_str(&lower_hex).unwrap());
    }

    #[test]
    fn test_h256_fmt_lower_hex() {
        let value: H256 = alloy::primitives::B256::random().into();
        let lower_hex = value.to_hex_str();
        assert!(lower_hex.starts_with("0x"));
        assert_eq!(value, H256::from_hex_str(&lower_hex).unwrap());
    }

    #[test]
    fn test_h64_transparent_serde_serialization() {
        let value: H64 = alloy::primitives::B64::random().into();

        let encoded_value = serde_json::json!(&value);
        let decoded_primitive: alloy::primitives::B64 =
            serde_json::from_value(encoded_value).unwrap();
        let encoded_primitive = serde_json::json!(&decoded_primitive);
        let decoded_value: H64 = serde_json::from_value(encoded_primitive).unwrap();

        assert_eq!(value, decoded_value);
    }

    #[test]
    fn test_h160_transparent_serde_serialization() {
        let value: H160 = alloy::primitives::Address::random().into();

        let encoded_value = serde_json::json!(&value);
        let decoded_primitive: alloy::primitives::Address =
            serde_json::from_value(encoded_value).unwrap();
        let encoded_primitive = serde_json::json!(&decoded_primitive);
        let decoded_value: H160 = serde_json::from_value(encoded_primitive).unwrap();

        assert_eq!(value, decoded_value);
    }

    #[test]
    fn test_h256_transparent_serde_serialization() {
        let value: H256 = alloy::primitives::B256::random().into();

        let encoded_value = serde_json::json!(&value);
        let decoded_primitive: alloy::primitives::B256 =
            serde_json::from_value(encoded_value).unwrap();
        let encoded_primitive = serde_json::json!(&decoded_primitive);
        let decoded_value: H256 = serde_json::from_value(encoded_primitive).unwrap();

        assert_eq!(value, decoded_value);
    }

    #[test]
    fn test_should_bincode_h160() {
        let value = H160::from_hex_str("0xbf380c52c18d5ead99ea719b6fcfbba551df2f7f")
            .expect("valid address");

        let encoded = bincode::serialize(&value).expect("serialization failed");
        let decoded: H160 = bincode::deserialize(&encoded).expect("deserialization failed");

        assert_eq!(value, decoded);
    }

    #[test]
    fn test_should_bincode_h64() {
        let value = H64::new(alloy::primitives::B64::random());

        let encoded = bincode::serialize(&value).expect("serialization failed");
        let decoded: H64 = bincode::deserialize(&encoded).expect("deserialization failed");

        assert_eq!(value, decoded);
    }

    #[test]
    fn test_should_bincode_h256() {
        let value = H256::new(alloy::primitives::B256::random());

        let encoded = bincode::serialize(&value).expect("serialization failed");
        let decoded: H256 = bincode::deserialize(&encoded).expect("deserialization failed");

        assert_eq!(value, decoded);
    }
}
