use std::borrow::Cow;
use std::fmt;
use std::ops::{Add, AddAssign, Mul, Sub};
use std::rc::Rc;
use std::str::FromStr;

use candid::types::{Type, TypeInner};
use candid::{CandidType, Deserialize, Nat};
use ic_stable_structures::{BoundedStorable, Storable};
use num::BigUint;
use serde::Serialize;

#[derive(Debug, Default, Clone, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
#[serde(transparent)]
pub struct U256(pub ethereum_types::U256);

#[derive(
    Debug, Default, Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize, Hash,
)]
#[serde(transparent)]
pub struct U64(pub ethereum_types::U64);

impl U256 {
    pub const BYTE_SIZE: usize = 32;

    pub fn new(value: ethereum_types::U256) -> Self {
        Self(value)
    }

    pub fn max_value() -> Self {
        Self(ethereum_types::U256::max_value())
    }

    pub fn from_hex_str(mut s: &str) -> Result<Self, String> {
        if s.starts_with("0x") || s.starts_with("0X") {
            s = &s[2..]
        }
        ethereum_types::U256::from_str(s)
            .map_err(|e| e.to_string())
            .map(Into::into)
    }

    pub fn to_hex_str(&self) -> String {
        format!("0x{self:x}")
    }

    pub const fn zero() -> Self {
        Self(ethereum_types::U256::zero())
    }

    pub const fn one() -> Self {
        Self(ethereum_types::U256::one())
    }

    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    pub fn to_big_endian(&self) -> Vec<u8> {
        let mut buffer = vec![0; 32];
        self.0.to_big_endian(&mut buffer);
        buffer
    }

    pub fn from_big_endian(slice: &[u8]) -> Self {
        Self(ethereum_types::U256::from_big_endian(slice))
    }

    pub fn to_little_endian(&self) -> Vec<u8> {
        let mut buffer = vec![0; 32];
        self.0.to_little_endian(&mut buffer);
        buffer
    }

    pub fn from_little_endian(slice: &[u8]) -> Self {
        Self(ethereum_types::U256::from_little_endian(slice))
    }

    pub fn checked_add(&self, rhs: &Self) -> Option<Self> {
        self.0.checked_add(rhs.0).map(Self)
    }

    pub fn checked_sub(&self, rhs: &Self) -> Option<Self> {
        self.0.checked_sub(rhs.0).map(Self)
    }

    pub fn checked_div(&self, rhs: &Self) -> Option<Self> {
        self.0.checked_div(rhs.0).map(Self)
    }

    pub fn checked_mul(&self, rhs: &Self) -> Option<Self> {
        self.0.checked_mul(rhs.0).map(Self)
    }
}

impl U64 {
    pub const BYTE_SIZE: usize = 8;

    pub fn new(value: ethereum_types::U64) -> Self {
        Self(value)
    }

    pub fn max_value() -> Self {
        Self(ethereum_types::U64::max_value())
    }

    pub fn from_hex_str(mut s: &str) -> Result<Self, String> {
        if s.starts_with("0x") || s.starts_with("0X") {
            s = &s[2..]
        }
        ethereum_types::U64::from_str(s)
            .map_err(|e| e.to_string())
            .map(Into::into)
    }

    pub fn to_hex_str(&self) -> String {
        format!("0x{self:x}")
    }

    pub const fn zero() -> Self {
        Self(ethereum_types::U64::zero())
    }

    pub const fn one() -> Self {
        Self(ethereum_types::U64::one())
    }

    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    pub fn to_big_endian(&self) -> Vec<u8> {
        let mut buffer = vec![0; 8];
        self.0.to_big_endian(&mut buffer);
        buffer
    }

    pub fn from_big_endian(slice: &[u8]) -> Self {
        Self(ethereum_types::U64::from_big_endian(slice))
    }

    pub fn to_little_endian(&self) -> Vec<u8> {
        let mut buffer = vec![0; 8];
        self.0.to_little_endian(&mut buffer);
        buffer
    }

    pub fn from_little_endian(slice: &[u8]) -> Self {
        Self(ethereum_types::U64::from_little_endian(slice))
    }
}

impl From<ethereum_types::U64> for U64 {
    fn from(v: ethereum_types::U64) -> Self {
        Self(v)
    }
}

impl From<U64> for ethereum_types::U64 {
    fn from(value: U64) -> Self {
        value.0
    }
}

impl From<ethereum_types::U256> for U256 {
    fn from(v: ethereum_types::U256) -> Self {
        Self(v)
    }
}

impl From<U256> for ethereum_types::U256 {
    fn from(value: U256) -> Self {
        value.0
    }
}

impl TryFrom<&Nat> for U256 {
    type Error = &'static str;

    fn try_from(v: &Nat) -> Result<Self, Self::Error> {
        let bytes = v.0.to_bytes_be();
        if bytes.len() > 32 {
            return Err("failed to convert too big Nat into U256");
        }

        Ok(Self::from_big_endian(&bytes))
    }
}

impl From<&U256> for Nat {
    fn from(value: &U256) -> Self {
        Nat(BigUint::from_bytes_be(&value.to_big_endian()))
    }
}

impl From<usize> for U64 {
    fn from(value: usize) -> Self {
        Self(value.into())
    }
}

impl From<U64> for usize {
    fn from(value: U64) -> Self {
        value.0.as_usize()
    }
}

impl From<u64> for U64 {
    fn from(value: u64) -> Self {
        Self(value.into())
    }
}
impl From<U64> for u64 {
    fn from(value: U64) -> Self {
        value.0.as_u64()
    }
}

impl From<usize> for U256 {
    fn from(value: usize) -> Self {
        Self(value.into())
    }
}

impl From<u64> for U256 {
    fn from(value: u64) -> Self {
        Self(value.into())
    }
}

impl From<u128> for U256 {
    fn from(value: u128) -> Self {
        Self(value.into())
    }
}

impl TryFrom<U256> for u128 {
    type Error = &'static str;

    fn try_from(value: U256) -> Result<Self, Self::Error> {
        value.0.try_into()
    }
}

// Implement manually because derive_more implementation does not work as expected
impl Mul for U256 {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        Self::new(self.0 * rhs.0)
    }
}

impl<'a, 'b> Mul<&'b U256> for &'a U256 {
    type Output = U256;

    fn mul(self, rhs: &'b U256) -> U256 {
        U256::new(self.0 * rhs.0)
    }
}

impl Add for U256 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self::new(self.0 + rhs.0)
    }
}

impl<'a, 'b> Add<&'b U256> for &'a U256 {
    type Output = U256;

    fn add(self, rhs: &'b U256) -> U256 {
        U256::new(self.0 + rhs.0)
    }
}

impl AddAssign for U256 {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0
    }
}

impl Sub for U256 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self::new(self.0 - rhs.0)
    }
}

impl<'a, 'b> Sub<&'b U256> for &'a U256 {
    type Output = U256;

    fn sub(self, rhs: &'b U256) -> U256 {
        U256::new(self.0 - rhs.0)
    }
}

impl Add for U64 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self::new(self.0 + rhs.0)
    }
}

impl AddAssign for U64 {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0
    }
}

impl Sub for U64 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self::new(self.0 - rhs.0)
    }
}

impl rlp::Encodable for U256 {
    fn rlp_append(&self, s: &mut rlp::RlpStream) {
        self.0.rlp_append(s);
    }
}

impl rlp::Decodable for U256 {
    fn decode(rlp: &rlp::Rlp) -> Result<Self, rlp::DecoderError> {
        ethereum_types::U256::decode(rlp).map(Into::into)
    }
}

impl fmt::Display for U256 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::LowerHex for U256 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl rlp::Encodable for U64 {
    fn rlp_append(&self, s: &mut rlp::RlpStream) {
        self.0.rlp_append(s);
    }
}

impl rlp::Decodable for U64 {
    fn decode(rlp: &rlp::Rlp) -> Result<Self, rlp::DecoderError> {
        ethereum_types::U64::decode(rlp).map(Into::into)
    }
}

impl fmt::Display for U64 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::LowerHex for U64 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Storable for U256 {
    fn to_bytes(&self) -> std::borrow::Cow<'_, [u8]> {
        self.to_big_endian().into()
    }

    fn from_bytes(bytes: Cow<'_, [u8]>) -> Self {
        Self::from_big_endian(bytes.as_ref())
    }
}

impl BoundedStorable for U256 {
    const MAX_SIZE: u32 = 32;
    const IS_FIXED_SIZE: bool = true;
}

impl CandidType for U64 {
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

impl CandidType for U256 {
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

// TODO::https://infinityswap.atlassian.net/browse/EPROD-552
// We should move to alloy-primitives crates

impl From<alloy_primitives::U256> for U256 {
    fn from(value: alloy_primitives::U256) -> Self {
        U256::from_little_endian(value.as_le_slice())
    }
}

impl From<U256> for alloy_primitives::U256 {
    fn from(value: U256) -> Self {
        let mut bytes = [0u8; U256::BYTE_SIZE];
        value.0.to_little_endian(&mut bytes);
        alloy_primitives::U256::from_le_bytes(bytes)
    }
}

impl From<alloy_primitives::U64> for U64 {
    fn from(value: alloy_primitives::U64) -> Self {
        U64::from_little_endian(value.as_le_slice())
    }
}

impl From<U64> for alloy_primitives::U64 {
    fn from(value: U64) -> Self {
        let mut bytes = [0u8; U64::BYTE_SIZE];
        value.0.to_little_endian(&mut bytes);
        alloy_primitives::U64::from_le_bytes(bytes)
    }
}

#[cfg(test)]
mod tests {

    use candid::{Decode, Encode};

    use super::*;

    #[test]
    fn test_storable_u256() {
        let value = ethereum_types::U256::from(rand::random::<u128>());
        let u256: U256 = value.into();

        let serialized = u256.to_bytes();
        let deserialized = U256::from_bytes(serialized);

        assert_eq!(u256, deserialized);
    }

    #[test]
    fn test_from_nat() {
        let nat = Nat::from(rand::random::<u128>());
        let u256: U256 = (&nat).try_into().unwrap();
        let nat_from_u256: Nat = (&u256).into();
        assert_eq!(nat, nat_from_u256);
    }

    #[test]
    fn test_from_too_big_nat() {
        let nat: Nat = Nat::from(&U256::max_value()) + 1;
        U256::try_from(&nat).unwrap_err();
    }

    #[test]
    fn test_to_nat() {
        let u256 = U256::from(rand::random::<u128>());
        let nat: Nat = (&u256).into();
        let u256_from_nat: U256 = (&nat).try_into().unwrap();
        assert_eq!(u256, u256_from_nat);
    }

    #[test]
    fn test_u256_little_endian_bytes() {
        let u256 = U256::from(rand::random::<u128>());
        let u256_from = U256::from_little_endian(&u256.to_little_endian());
        assert_eq!(u256, u256_from);
    }

    #[test]
    fn test_u256_big_endian_bytes() {
        let u256 = U256::from(rand::random::<u128>());
        let u256_from = U256::from_big_endian(&u256.to_big_endian());
        assert_eq!(u256, u256_from);
    }

    #[test]
    fn test_u256_is_zero() {
        assert!(U256::default().is_zero());
        assert!(!U256::from(1u64).is_zero());
        assert!(!U256::from(100u64).is_zero());
    }

    #[test]
    fn test_u64_little_endian_bytes() {
        let u64 = U64::from(rand::random::<u64>());
        let u64_from = U64::from_little_endian(&u64.to_little_endian());
        assert_eq!(u64, u64_from);
    }

    #[test]
    fn test_u64_big_endian_bytes() {
        let u64 = U64::from(rand::random::<u64>());
        let u64_from = U64::from_big_endian(&u64.to_big_endian());
        assert_eq!(u64, u64_from);
    }

    #[test]
    fn test_u64_is_zero() {
        assert!(U64::default().is_zero());
        assert!(!U64::from(1u64).is_zero());
        assert!(!U64::from(100u64).is_zero());
    }

    #[test]
    fn test_candid_type_u64() {
        let value = ethereum_types::U64::from(rand::random::<u64>());
        let u64: U64 = value.into();

        let encoded = Encode!(&u64).unwrap();
        let decoded = Decode!(&encoded, U64).unwrap();

        assert_eq!(u64, decoded);
    }

    #[test]
    fn test_candid_type_u256() {
        let value = ethereum_types::U256::from(rand::random::<u128>());
        let u256: U256 = value.into();

        let encoded = Encode!(&u256).unwrap();
        let decoded = Decode!(&encoded, U256).unwrap();

        assert_eq!(u256, decoded);
    }

    #[test]
    fn test_u256_from_hex_should_fail_long_length() {
        assert!(U256::from_hex_str(
            "18201820182018201820182018201820182018201820182018201820182018212"
        )
        .is_err());
    }

    #[test]
    fn test_u256_from_hex_should_fail_invalid_char() {
        assert!(U256::from_hex_str(
            "18201820182018201820182018201820182018201820182018201820182018g"
        )
        .is_err());
    }

    #[test]
    fn test_u256_fmt_lower_hex() {
        let value: U256 = ethereum_types::U256::from(rand::random::<u128>()).into();
        let lower_hex = value.to_hex_str();
        assert!(lower_hex.starts_with("0x"));
        assert_eq!(value, U256::from_hex_str(&lower_hex).unwrap());
    }

    #[test]
    fn test_u256_from_hex_should_succeed() {
        assert_eq!(U256::from(0u64), U256::from_hex_str("00").unwrap());
        assert_eq!(U256::from(1u64), U256::from_hex_str("01").unwrap());
        assert_eq!(U256::from(255u64), U256::from_hex_str("ff").unwrap());
        assert_eq!(
            U256::from(2074343815918867987178857765017879333u128),
            U256::from_hex_str("18F810BD8895AA66364CBDD91A20325").unwrap()
        );

        assert_eq!(
            U256::from(0x0123456789abcdefu128),
            U256::from_hex_str("0123456789abcdef").unwrap()
        );
        assert_eq!(
            U256::max_value(),
            U256::from_hex_str("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff")
                .unwrap()
        );
    }

    #[test]
    fn test_u64_operations() {
        let a = U64::from(101u64);
        let b = U64::from(10u64);

        let add = U64::from(111u64);
        let sub = U64::from(91u64);

        assert_eq!(add, a + b);
        assert_eq!(sub, a - b);

        let mut c = a;
        c += b;
        assert_eq!(add, c);
    }

    #[test]
    fn test_u256_operations() {
        let a = U256::from(101u64);
        let b = U256::from(10u64);

        let add = U256::from(111u64);
        let sub = U256::from(91u64);
        let div = U256::from(10u64);
        let mul = U256::from(1010u64);

        assert_eq!(add, &a + &b);
        assert_eq!(mul, &a * &b);
        assert_eq!(sub, &a - &b);

        assert_eq!(add, &a + &b);
        assert_eq!(mul, &a * &b);
        assert_eq!(sub, &a - &b);

        // checked operations
        let checked_add = a.checked_add(&b);
        let checked_sub = a.checked_sub(&b);
        let checked_div = a.checked_div(&b);
        let checked_mul = a.checked_mul(&b);

        assert_eq!(checked_add, Some(add.clone()));
        assert_eq!(checked_sub, Some(sub));
        assert_eq!(checked_mul, Some(mul));
        assert_eq!(checked_div, Some(div));

        let add_overflow = U256::max_value().checked_add(&a);
        let sub_overflow = U256::zero().checked_sub(&a);

        assert!(add_overflow.is_none());
        assert!(sub_overflow.is_none());

        let mut c = a;
        c += b;
        assert_eq!(add, c);
    }

    #[test]
    fn test_u256_transparent_serde_serialization() {
        let value: U256 = ethereum_types::U256::from(rand::random::<u128>()).into();

        let encoded_value = serde_json::json!(&value);
        let decoded_primitive: ethereum_types::U256 =
            serde_json::from_value(encoded_value).unwrap();
        let encoded_primitive = serde_json::json!(&decoded_primitive);
        let decoded_value: U256 = serde_json::from_value(encoded_primitive).unwrap();

        assert_eq!(value, decoded_value);
    }

    #[test]
    fn test_u64_from_hex_should_fail_odd_length() {
        assert!(U64::from_hex_str(
            "182018201820182018201820182018201820182018201820182018201820182"
        )
        .is_err());
    }

    #[test]
    fn test_u64_from_hex_should_fail_long_length() {
        assert!(U64::from_hex_str(
            "18201820182018201820182018201820182018201820182018201820182018212"
        )
        .is_err());
    }

    #[test]
    fn test_u64_from_hex_should_fail_invalid_char() {
        assert!(U64::from_hex_str(
            "18201820182018201820182018201820182018201820182018201820182018g"
        )
        .is_err());
    }

    #[test]
    fn test_u64_fmt_lower_hex() {
        let value: U64 = ethereum_types::U64::from(rand::random::<u64>()).into();
        let lower_hex = value.to_hex_str();
        assert!(lower_hex.starts_with("0x"));
        assert_eq!(value, U64::from_hex_str(&lower_hex).unwrap());
    }

    #[test]
    fn test_u64_from_hex_should_succeed() {
        assert_eq!(U64::from(0u64), U64::from_hex_str("00").unwrap());
        assert_eq!(U64::from(1u64), U64::from_hex_str("0x01").unwrap());
        assert_eq!(U64::from(255u64), U64::from_hex_str("ff").unwrap());
        assert_eq!(
            U64::from(72057594037927936u64),
            U64::from_hex_str("100000000000000").unwrap()
        );

        assert_eq!(
            U64::from(0x0123456789abcdefu64),
            U64::from_hex_str("0123456789abcdef").unwrap()
        );
        assert_eq!(
            U64::max_value(),
            U64::from_hex_str("0Xffffffffffffffff").unwrap()
        );
    }

    #[test]
    fn test_u64_transparent_serde_serialization() {
        let value: U64 = ethereum_types::U64::from(rand::random::<u64>()).into();

        let encoded_value = serde_json::json!(&value);
        let decoded_primitive: ethereum_types::U64 = serde_json::from_value(encoded_value).unwrap();
        let encoded_primitive = serde_json::json!(&decoded_primitive);
        let decoded_value: U64 = serde_json::from_value(encoded_primitive).unwrap();

        assert_eq!(value, decoded_value);
    }

    #[test]
    fn test_alloy_u256_roundtrip() {
        let value: U256 = ethereum_types::U256::from(rand::random::<u128>()).into();

        let alloy_u256: alloy_primitives::U256 = value.clone().into();
        let decoded_value: U256 = alloy_u256.into();

        assert_eq!(value, decoded_value);
    }

    #[test]
    fn test_alloy_u64_roundtrip() {
        let value: U64 = ethereum_types::U64::from(rand::random::<u64>()).into();

        let alloy_u64: alloy_primitives::U64 = value.into();
        let decoded_value: U64 = alloy_u64.into();

        assert_eq!(value, decoded_value);
    }
}
