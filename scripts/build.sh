#!/bin/bash
set -e
export RUST_BACKTRACE=full
export WASM_DIR=./target/artifact

build_iceth_client_test_canisters() {

    TAG=v0.1.1
    ICETH_URL=https://github.com/infinity-swap/iceth/releases/download/${TAG}/iceth-${TAG}.tar.gz
    echo "Downloading $ICETH_URL"
    curl -fsSL $ICETH_URL | tar -xz -C "$WASM_DIR"

    echo "Building iceth-client test canister"
    cargo run -p iceth-client-test-canister --features export-api --release > $WASM_DIR/iceth-client-test-canister.did
    cargo build -p iceth-client-test-canister --target wasm32-unknown-unknown --features export-api --release
    ic-wasm target/wasm32-unknown-unknown/release/iceth-client-test-canister.wasm -o $WASM_DIR/iceth-client-test-canister.wasm shrink
}

build_ic_sign_test_canister() {
    echo "Building ic-sign-client test canisters"
    cargo run -p ic-sign-test-canister --features "export-api" > $WASM_DIR/ic-sign-test-canister.did
    cargo build -p ic-sign-test-canister --target wasm32-unknown-unknown --release --features "export-api"
    ic-wasm target/wasm32-unknown-unknown/release/ic-sign-test-canister.wasm -o $WASM_DIR/ic-sign-test-canister.wasm shrink
}

main() {
    mkdir -p $WASM_DIR

    build_iceth_client_test_canisters
    build_ic_sign_test_canister

}

main "$@"
