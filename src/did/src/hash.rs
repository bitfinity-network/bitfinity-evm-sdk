use std::borrow::Cow;
use std::fmt;
use std::rc::Rc;

use candid::types::{Type, TypeInner};
use candid::{CandidType, Deserialize};
use derive_more::Display;
use ethers_core::types::NameOrAddress;
use ic_stable_structures::{Bound, Bounded, Storable};
use serde::Serialize;

#[derive(
    Debug, Default, Clone, PartialOrd, Ord, Eq, PartialEq, Serialize, Deserialize, Display, Hash,
)]
#[serde(transparent)]
pub struct Hash<T>(pub T);

///Fixed-size uninterpreted hash type with 8 bytes (64 bits) size.
pub type H64 = Hash<ethereum_types::H64>;
///Fixed-size uninterpreted hash type with 20 bytes (160 bits) size.
pub type H160 = Hash<ethereum_types::H160>;
///Fixed-size uninterpreted hash type with 32 bytes (256 bits) size.
pub type H256 = Hash<ethereum_types::H256>;

pub fn from_hex_str<const SIZE: usize>(mut s: &str) -> Result<[u8; SIZE], hex::FromHexError> {
    if s.starts_with("0x") || s.starts_with("0X") {
        s = &s[2..];
    }

    let mut result = [0u8; SIZE];
    hex::decode_to_slice(s, &mut result).and(Ok(result))
}

impl H64 {
    pub const BYTE_SIZE: usize = 8;

    pub fn new(value: ethereum_types::H64) -> Self {
        Self(value)
    }

    pub fn from_slice(slice: &[u8]) -> Self {
        Self(ethereum_types::H64::from_slice(slice))
    }

    pub fn from_hex_str(s: &str) -> Result<Self, hex::FromHexError> {
        Ok(Self(ethereum_types::H64::from(from_hex_str::<8>(s)?)))
    }

    pub fn to_hex_str(&self) -> String {
        format!("0x{self:x}")
    }

    pub const fn zero() -> Self {
        Self(ethereum_types::H64::zero())
    }
}

impl H160 {
    pub const BYTE_SIZE: usize = 20;

    pub fn new(value: ethereum_types::H160) -> Self {
        Self(value)
    }

    pub fn from_slice(slice: &[u8]) -> Self {
        Self(ethereum_types::H160::from_slice(slice))
    }

    pub fn from_hex_str(s: &str) -> Result<Self, hex::FromHexError> {
        Ok(Self(ethereum_types::H160::from(from_hex_str::<20>(s)?)))
    }

    pub fn to_hex_str(&self) -> String {
        format!("0x{self:x}")
    }

    pub const fn zero() -> Self {
        Self(ethereum_types::H160::zero())
    }
}

impl H256 {
    pub const BYTE_SIZE: usize = 32;

    pub fn new(value: ethereum_types::H256) -> Self {
        Self(value)
    }

    pub fn from_slice(slice: &[u8]) -> Self {
        Self(ethereum_types::H256::from_slice(slice))
    }

    pub fn from_hex_str(s: &str) -> Result<Self, hex::FromHexError> {
        Ok(Self(ethereum_types::H256::from(from_hex_str::<32>(s)?)))
    }

    pub fn to_hex_str(&self) -> String {
        format!("0x{self:x}")
    }

    pub const fn zero() -> Self {
        Self(ethereum_types::H256::zero())
    }
}

impl Storable for H64 {
    fn to_bytes(&self) -> Cow<'_, [u8]> {
        self.0.as_ref().into()
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Self(ethereum_types::H64::from_slice(bytes.as_ref()))
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: H64::BYTE_SIZE as u32,
        is_fixed_size: true,
    };
}

impl Storable for H160 {
    fn to_bytes(&self) -> Cow<'_, [u8]> {
        self.0.as_ref().into()
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Self(ethereum_types::H160::from_slice(bytes.as_ref()))
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: H160::BYTE_SIZE as u32,
        is_fixed_size: true,
    };
}

impl Storable for H256 {
    fn to_bytes(&self) -> Cow<'_, [u8]> {
        self.0.as_ref().into()
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Self(ethereum_types::H256::from_slice(bytes.as_ref()))
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

impl rlp::Encodable for H64 {
    fn rlp_append(&self, s: &mut rlp::RlpStream) {
        self.0.rlp_append(s);
    }
}

impl rlp::Decodable for H64 {
    fn decode(rlp: &rlp::Rlp) -> Result<Self, rlp::DecoderError> {
        ethereum_types::H64::decode(rlp).map(Into::into)
    }
}

impl rlp::Encodable for H160 {
    fn rlp_append(&self, s: &mut rlp::RlpStream) {
        self.0.rlp_append(s);
    }
}

impl rlp::Decodable for H160 {
    fn decode(rlp: &rlp::Rlp) -> Result<Self, rlp::DecoderError> {
        ethereum_types::H160::decode(rlp).map(Into::into)
    }
}

impl rlp::Encodable for H256 {
    fn rlp_append(&self, s: &mut rlp::RlpStream) {
        self.0.rlp_append(s);
    }
}

impl rlp::Decodable for H256 {
    fn decode(rlp: &rlp::Rlp) -> Result<Self, rlp::DecoderError> {
        ethereum_types::H256::decode(rlp).map(Into::into)
    }
}

impl Bounded for H64 {
    const MIN: H64 = Hash::<ethereum_types::H64>(ethereum_types::H64([u8::MIN; 8]));
    const MAX: H64 = Hash::<ethereum_types::H64>(ethereum_types::H64([u8::MAX; 8]));
}

impl Bounded for H160 {
    const MIN: H160 = Hash::<ethereum_types::H160>(ethereum_types::H160([u8::MIN; 20]));
    const MAX: H160 = Hash::<ethereum_types::H160>(ethereum_types::H160([u8::MAX; 20]));
}

impl Bounded for H256 {
    const MIN: H256 = Hash::<ethereum_types::H256>(ethereum_types::H256([u8::MIN; 32]));
    const MAX: H256 = Hash::<ethereum_types::H256>(ethereum_types::H256([u8::MAX; 32]));
}

impl From<H64> for ethereum_types::H64 {
    fn from(value: H64) -> Self {
        value.0
    }
}

impl From<H160> for ethereum_types::H160 {
    fn from(value: H160) -> Self {
        value.0
    }
}

impl From<H256> for ethereum_types::H256 {
    fn from(value: H256) -> Self {
        value.0
    }
}

impl From<ethereum_types::H64> for H64 {
    fn from(value: ethereum_types::H64) -> Self {
        Hash(value)
    }
}

impl From<ethereum_types::H160> for H160 {
    fn from(value: ethereum_types::H160) -> Self {
        Hash(value)
    }
}

impl From<ethereum_types::H256> for H256 {
    fn from(value: ethereum_types::H256) -> Self {
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

impl From<NameOrAddress> for H160 {
    fn from(s: NameOrAddress) -> Self {
        match s {
            NameOrAddress::Address(a) => a.into(),
            // We don't have a way to resolve names to addresses, so we just return 0, I am not sure if we support names at all.
            NameOrAddress::Name(_) => H160::zero(),
        }
    }
}

impl fmt::LowerHex for H64 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::LowerHex for H160 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::LowerHex for H256 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

// TODO::https://infinityswap.atlassian.net/browse/EPROD-552
// We should move to alloy-primitives crates

impl From<alloy_primitives::Address> for H160 {
    fn from(value: alloy_primitives::Address) -> Self {
        H160::from_slice(value.as_slice())
    }
}

impl From<H160> for alloy_primitives::Address {
    fn from(value: H160) -> Self {
        alloy_primitives::Address::from_slice(value.0.as_bytes())
    }
}

impl From<alloy_primitives::B64> for H64 {
    fn from(value: alloy_primitives::B64) -> Self {
        H64::from_slice(value.as_slice())
    }
}

impl From<H64> for alloy_primitives::B64 {
    fn from(value: H64) -> Self {
        alloy_primitives::B64::from_slice(value.0.as_bytes())
    }
}

impl From<alloy_primitives::B256> for H256 {
    fn from(value: alloy_primitives::B256) -> Self {
        H256::from_slice(value.as_slice())
    }
}

impl From<H256> for alloy_primitives::B256 {
    fn from(value: H256) -> Self {
        alloy_primitives::B256::from_slice(value.0.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use candid::{Decode, Encode};
    use ethers_core::types::NameOrAddress;
    use ic_stable_structures::Storable;
    use rlp::Encodable;

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
        assert_eq!(val.unwrap_err(), hex::FromHexError::InvalidStringLength);

        let (_, str_val) = generate_hex_str(50);
        let val = H256::from_hex_str(&str_val);
        assert_eq!(val.unwrap_err(), hex::FromHexError::InvalidStringLength);
    }

    #[test]
    fn test_rlp_encoding_decoding_h256() {
        let (hex_val, _) = generate_hex_str(32);
        let value = H256::from_slice(&hex_val);
        let mut stream = rlp::RlpStream::new();
        value.rlp_append(&mut stream);
        let encoded = stream.out();

        let decoded = rlp::decode::<H256>(&encoded).unwrap();
        assert_eq!(value, decoded);
    }

    #[test]
    fn test_rlp_encoding_decoding_h160() {
        let (hex_val, _) = generate_hex_str(20);
        let value = H160::from_slice(&hex_val);
        let mut stream = rlp::RlpStream::new();
        value.rlp_append(&mut stream);
        let encoded = stream.out();

        let decoded = rlp::decode::<H160>(&encoded).unwrap();
        assert_eq!(value, decoded);
    }

    #[test]
    fn test_rlp_encoding_decoding_h64() {
        let (hex_val, _) = generate_hex_str(8);
        let value = H64::from_slice(&hex_val);
        let mut stream = rlp::RlpStream::new();
        value.rlp_append(&mut stream);
        let encoded = stream.out();

        let decoded = rlp::decode::<H64>(&encoded).unwrap();
        assert_eq!(value, decoded);
    }

    #[test]
    fn test_h160_from_address() {
        let (hex_val, hex_string) = generate_hex_str(20);
        let address = NameOrAddress::Address(ethereum_types::H160::from_slice(&hex_val));
        let h160 = H160::from_hex_str(&hex_string).unwrap();
        assert_eq!(h160, address.into());
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
        let h64 = H64::new(ethereum_types::H64::random());

        let encoded = serde_json::json!(&h64);
        let decoded = serde_json::from_value(encoded).unwrap();

        assert_eq!(h64, decoded);
    }

    #[test]
    fn test_serde_h160() {
        let h160 = H160::new(ethereum_types::H160::random());

        let encoded = serde_json::json!(&h160);
        let decoded = serde_json::from_value(encoded).unwrap();

        assert_eq!(h160, decoded);
    }

    #[test]
    fn test_serde_h256() {
        let h256 = H256::new(ethereum_types::H256::random());

        let encoded = serde_json::json!(&h256);
        let decoded = serde_json::from_value(encoded).unwrap();

        assert_eq!(h256, decoded);
    }

    #[test]
    fn test_h64_fmt_lower_hex() {
        let value: H64 = ethereum_types::H64::random().into();
        let lower_hex = value.to_hex_str();
        assert!(lower_hex.starts_with("0x"));
        assert_eq!(value, H64::from_hex_str(&lower_hex).unwrap());
    }

    #[test]
    fn test_h160_fmt_lower_hex() {
        let value: H160 = ethereum_types::H160::random().into();
        let lower_hex = value.to_hex_str();
        assert!(lower_hex.starts_with("0x"));
        assert_eq!(value, H160::from_hex_str(&lower_hex).unwrap());
    }

    #[test]
    fn test_h256_fmt_lower_hex() {
        let value: H256 = ethereum_types::H256::random().into();
        let lower_hex = value.to_hex_str();
        assert!(lower_hex.starts_with("0x"));
        assert_eq!(value, H256::from_hex_str(&lower_hex).unwrap());
    }

    #[test]
    fn test_h64_transparent_serde_serialization() {
        let value: H64 = ethereum_types::H64::random().into();

        let encoded_value = serde_json::json!(&value);
        let decoded_primitive: ethereum_types::H64 = serde_json::from_value(encoded_value).unwrap();
        let encoded_primitive = serde_json::json!(&decoded_primitive);
        let decoded_value: H64 = serde_json::from_value(encoded_primitive).unwrap();

        assert_eq!(value, decoded_value);
    }

    #[test]
    fn test_h160_transparent_serde_serialization() {
        let value: H160 = ethereum_types::H160::random().into();

        let encoded_value = serde_json::json!(&value);
        let decoded_primitive: ethereum_types::H160 =
            serde_json::from_value(encoded_value).unwrap();
        let encoded_primitive = serde_json::json!(&decoded_primitive);
        let decoded_value: H160 = serde_json::from_value(encoded_primitive).unwrap();

        assert_eq!(value, decoded_value);
    }

    #[test]
    fn test_h256_transparent_serde_serialization() {
        let value: H256 = ethereum_types::H256::random().into();

        let encoded_value = serde_json::json!(&value);
        let decoded_primitive: ethereum_types::H256 =
            serde_json::from_value(encoded_value).unwrap();
        let encoded_primitive = serde_json::json!(&decoded_primitive);
        let decoded_value: H256 = serde_json::from_value(encoded_primitive).unwrap();

        assert_eq!(value, decoded_value);
    }

    #[test]
    fn test_alloy_address_roundtrip() {
        let value: H160 = ethereum_types::H160::random().into();

        let alloy_address = alloy_primitives::Address::from(value.clone());
        let decoded_value = H160::from(alloy_address);

        assert_eq!(value, decoded_value);
    }

    #[test]
    fn test_alloy_h256_roundtrip() {
        let value: H256 = ethereum_types::H256::random().into();

        let alloy_h256 = alloy_primitives::B256::from(value.clone());
        let decoded_value = H256::from(alloy_h256);

        assert_eq!(value, decoded_value);
    }

    #[test]
    fn test_alloy_h64_roundtrip() {
        let value: H64 = ethereum_types::H64::random().into();

        let alloy_h64 = alloy_primitives::B64::from(value.clone());
        let decoded_value = H64::from(alloy_h64);

        assert_eq!(value, decoded_value);
    }
}
