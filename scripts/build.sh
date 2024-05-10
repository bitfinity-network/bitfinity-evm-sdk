#!/bin/bash
set -e
set -x #echo on

export RUST_BACKTRACE=full
export WASM_DIR=./target/artifact

build_ic_sign_test_canister() {
    echo "Building ic-sign-client test canisters"
    cargo run -p ic-sign-test-canister --features "export-api" > $WASM_DIR/ic-sign-test-canister.did
    cargo build -p ic-sign-test-canister --target wasm32-unknown-unknown --release --features "export-api"
    ic-wasm target/wasm32-unknown-unknown/release/ic-sign-test-canister.wasm -o $WASM_DIR/ic-sign-test-canister.wasm shrink
    gzip -k "$WASM_DIR/ic-sign-test-canister.wasm" --force
}

main() {
    mkdir -p $WASM_DIR

    build_ic_sign_test_canister

}

main "$@"
