use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Returns the bytecode of the canister
pub fn get_test_canister_bytecode() -> Vec<u8> {
    static CANISTER_BYTECODE: OnceLock<Vec<u8>> = OnceLock::new();
    CANISTER_BYTECODE
        .get_or_init(|| load_wasm_bytecode_or_panic("ic-sign-test-canister.wasm.gz"))
        .to_owned()
}

fn load_wasm_bytecode_or_panic(wasm_name: &str) -> Vec<u8> {
    let path = get_path_to_wasm(wasm_name);

    let mut f = File::open(path).expect("File does not exists");

    let mut buffer = Vec::new();
    f.read_to_end(&mut buffer)
        .expect("Could not read file content");

    buffer
}

fn get_path_to_wasm(wasm_name: &str) -> PathBuf {
    const ARTIFACT_PATH: &str = "../../target/artifact/";
    // Get to the root of the project
    let wasm_path = format!("{}{}", ARTIFACT_PATH, wasm_name);
    println!("path: {wasm_path:?}");
    if Path::new(&wasm_path).exists() {
        wasm_path.into()
    } else {
        panic!("File {wasm_name} was not found in {ARTIFACT_PATH}");
    }
}
