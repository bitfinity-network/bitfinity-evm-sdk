#[macro_export]
macro_rules! construct_did_int {
    (U256) => {
        construct_did_int!(@inner U256, ethereum_types::U256, 32);
    };
    (U64) => {
        construct_did_int!(@inner U64, ethereum_types::U64, 8);
    };
    (@inner $name:ident, $inner:ty, $byte_size:expr) => {
        #[derive(
            Debug, Default, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize, Hash, From, Into,
        )]
        #[serde(transparent)]
        pub struct $name(pub $inner);

        impl $name {
            pub const BYTE_SIZE: usize = $byte_size;

            pub fn new(value: $inner) -> Self {
                Self(value)
            }

            pub fn max_value() -> Self {
                Self(<$inner>::max_value())
            }

            pub const fn zero() -> Self {
                Self(<$inner>::zero())
            }

            pub fn to_hex_str(&self) -> String {
                format!("0x{self:x}")
            }

            pub fn from_hex_str(s: &str) -> Result<Self, String> {
                if s.starts_with("0x") || s.starts_with("0X") {
                    <$inner>::from_str(&s[2..])
                } else {
                    <$inner>::from_str(s)
                }
                .map_err(|e| e.to_string())
                .map(Into::into)
            }

            pub const fn one() -> Self {
                Self(<$inner>::one())
            }

            pub fn is_zero(&self) -> bool {
                self.0.is_zero()
            }

            pub fn to_big_endian(&self) -> Vec<u8> {
                let mut buffer = vec![0; $byte_size];
                self.0.to_big_endian(&mut buffer);
                buffer
            }

            pub fn from_big_endian(slice: &[u8]) -> Self {
                Self(<$inner>::from_big_endian(slice))
            }

            pub fn to_little_endian(&self) -> Vec<u8> {
                let mut buffer = vec![0; $byte_size];
                self.0.to_little_endian(&mut buffer);
                buffer
            }

            pub fn from_little_endian(slice: &[u8]) -> Self {
                Self(<$inner>::from_little_endian(slice))
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

        impl rlp::Encodable for $name {
            fn rlp_append(&self, s: &mut rlp::RlpStream) {
                self.0.rlp_append(s);
            }
        }

        impl rlp::Decodable for $name {
            fn decode(rlp: &rlp::Rlp) -> Result<Self, rlp::DecoderError> {
                <$inner>::decode(rlp).map(Into::into)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                self.0.fmt(f)
            }
        }

        impl fmt::LowerHex for $name {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                self.0.fmt(f)
            }
        }

        impl CandidType for $name {
            fn _ty() -> candid::types::Type {
                candid::types::Type(Rc::new(candid::types::TypeInner::Text))
            }

            fn idl_serialize<S>(&self, serializer: S) -> Result<(), S::Error>
            where
                S: candid::types::Serializer,
            {
                serializer.serialize_text(&self.to_hex_str())
            }
        }
    };
}

#[macro_export]
macro_rules! impl_from_for_ethereum_type {
    ($type:ty, $inner:ty) => {
        impl From<$inner> for $type {
            fn from(value: $inner) -> Self {
                Self::new(value.into())
            }
        }

        impl From<$type> for $inner {
            fn from(value: $type) -> Self {
                value.0.as_$inner()
            }
        }
    };
}
