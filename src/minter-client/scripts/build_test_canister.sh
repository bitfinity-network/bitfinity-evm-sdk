#!/bin/bash

cargo run -p iceth-client-test-canister --features export-api --release > $WASM_DIR/iceth-client-test-canister.did
cargo build -p iceth-client-test-canister --target wasm32-unknown-unknown --features export-api --release
ic-wasm target/wasm32-unknown-unknown/release/iceth-client-test-canister.wasm \
    -o target/wasm32-unknown-unknown/release/iceth-client-test-canister-opt.wasm shrink
gzip -k target/wasm32-unknown-unknown/release/iceth-client-test-canister-opt.wasm --force
cp target/wasm32-unknown-unknown/release/iceth-client-test-canister-opt.wasm.gz $WASM_DIR/
