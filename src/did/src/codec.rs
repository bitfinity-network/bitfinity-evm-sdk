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