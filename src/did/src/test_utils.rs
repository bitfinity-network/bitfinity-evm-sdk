use std::collections::HashMap;
use std::fmt::Debug;

use candid::CandidType;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Reads all the files in a directory and parses them into serde_json::Value
pub fn read_all_files_to_json(path: &str) -> HashMap<String, Value> {
    let mut jsons = HashMap::new();
    for file in std::fs::read_dir(path).unwrap() {
        let file = file.unwrap();
        let filename = file
            .file_name()
            .to_str()
            .unwrap()
            .trim_end_matches(".json")
            .to_owned();
        let value: Value =
            serde_json::from_str(&std::fs::read_to_string(file.path()).unwrap()).unwrap();
        jsons.insert(filename, value);
    }
    jsons
}

pub fn test_json_roundtrip<T>(val: &T)
where
    for<'de> T: Serialize + Deserialize<'de> + PartialEq + Debug,
{
    let serialized = serde_json::to_string(val).unwrap();
    let restored: T = serde_json::from_str(&serialized).unwrap();

    assert_eq!(val, &restored);
}

pub fn test_candid_roundtrip<T>(val: &T)
where
    for<'de> T: CandidType + Deserialize<'de> + PartialEq + Debug,
{
    let serialized = candid::encode_one(val).unwrap();
    let restored: T = candid::decode_one(&serialized).unwrap();

    assert_eq!(val, &restored);
}
