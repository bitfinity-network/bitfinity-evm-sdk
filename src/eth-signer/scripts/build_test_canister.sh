#!/bin/bash

cargo run -p ic-sign-test-canister --features "export-api" > $WASM_DIR/ic-sign-test-canister.did
cargo build -p ic-sign-test-canister --target wasm32-unknown-unknown --release --features "export-api"
ic-wasm target/wasm32-unknown-unknown/release/ic-sign-test-canister.wasm -o target/wasm32-unknown-unknown/release/ic-sign-test-canister-opt.wasm shrink
gzip -k target/wasm32-unknown-unknown/release/ic-sign-test-canister-opt.wasm --force
cp target/wasm32-unknown-unknown/release/ic-sign-test-canister-opt.wasm.gz $WASM_DIR/