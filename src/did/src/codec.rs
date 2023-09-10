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

pub mod macro_utils {
    use ic_stable_structures::BoundedStorable;

    pub const fn is_fixed_size<T: BoundedStorable>(_: &T) -> bool {
        T::IS_FIXED_SIZE
    }

    pub const fn get_max_size<T: BoundedStorable>(_: &T) -> u32 {
        T::MAX_SIZE
    }
}

#[macro_export]
macro_rules! encode_fixed_storables {
    ($($values:expr),+) => {
        {
            let all_fixed_size = true $( && $crate::codec::macro_utils::is_fixed_size(&$values) )*;
            assert!(all_fixed_size);

            let size = 0 $( + $crate::codec::macro_utils::get_max_size(&$values))*;

            let mut result = std::vec::Vec::<u8>::with_capacity(size as _);
            $(
                result.extend_from_slice(&$values.to_bytes());
            )*

            assert_eq!(result.len(), size as usize);

            result
        }
    };
}

#[macro_export]
macro_rules! decode_fixed_storables {
    ($data:expr, $($types:ty),+) => {
        {
            #[allow(unused)]
            const ALL_FIXED_SIZE: bool = true $( && <$types as ic_stable_structures::BoundedStorable>::IS_FIXED_SIZE)*;
            assert!(ALL_FIXED_SIZE);

            (decode_fixed_storables!($data, 0, $($types,)*))
        }
    };

    ($data:expr, $offset:expr, $type:ty,) => {
        <$type>::from_bytes((&$data[$offset as usize..$offset + <$type as ic_stable_structures::BoundedStorable>::MAX_SIZE as usize]).into())
    };

    ($data:expr, $offset:expr, $type:ty, $($types:ty),+) => {
        $type::from_bytes($data[$offset..$offset + ty::MAX_SIZE]), decode_fixed_storables!($data, $offset + $type::MAX_SIZE, $($types)*)
    };
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use ic_stable_structures::{BoundedStorable, Storable};

    #[derive(PartialEq, Eq, Debug)]
    struct Type1([u8; 2]);

    impl Storable for Type1 {
        fn to_bytes(&self) -> Cow<[u8]> {
            Cow::Borrowed(&self.0)
        }

        fn from_bytes(bytes: Cow<[u8]>) -> Self {
            Self(bytes.as_ref().try_into().unwrap())
        }
    }

    impl BoundedStorable for Type1 {
        const MAX_SIZE: u32 = 2;

        const IS_FIXED_SIZE: bool = true;
    }

    #[test]
    fn check_single_type_roundtrip() {
        let value = Type1([0; 2]);
        let data = encode_fixed_storables!(value);
        let decoded = decode_fixed_storables!(data, Type1);

        assert_eq!(value, decoded);
    }
}
