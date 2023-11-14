#!/bin/bash
set -e

export WASM_DIR=./target/artifact

deploy_ic_sign_test_canister() {
    echo "Deploying ic-sign-test-canister artifacts..."
    dfx deploy ic-sign-test-canister
}

main() {

    dfx stop || killall dfx icx-proxy || true

    dfx start --clean --background
    # Wait for dfx started
    sleep 5

    deploy_ic_sign_test_canister
}

main "$@"
