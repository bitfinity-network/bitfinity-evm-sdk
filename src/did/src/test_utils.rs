use std::collections::HashMap;

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
