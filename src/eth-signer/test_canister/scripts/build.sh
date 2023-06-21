cargo run -p ic_sign_test_canister --features "export-api" > ./src/eth-signer/test_canister/test_canister.did
cargo build -p ic_sign_test_canister --target wasm32-unknown-unknown --release --features "export-api"
ic-wasm target/wasm32-unknown-unknown/release/ic_sign_test_canister.wasm -o target/wasm32-unknown-unknown/release/ic_sign_test_canister_opt.wasm shrink
gzip -k target/wasm32-unknown-unknown/release/ic_sign_test_canister_opt.wasm --force