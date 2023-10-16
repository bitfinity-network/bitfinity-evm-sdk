#!/bin/bash
set -e

export WASM_DIR=./target/artifact

deploy_iceth_client_test_canisters() {
    echo "Deploying iceth artifact..."
    dfx deploy iceth 
    
    echo "Deploying iceth-client artifact..."
    ICETH_PRINCIPAL=$(dfx canister id iceth)
    INIT_ARGS="record { url=\"https://testnet.bitfinity.network\"; iceth=principal \"$ICETH_PRINCIPAL\"; chain_id=355113}"
    dfx deploy iceth-client-test-canister --argument "$INIT_ARGS" -m reinstall -y
}

deploy_ic_sign_test_canister() {
    echo "Deploying ic-sign-test-canister artifacts..."
    dfx deploy ic-sign-test-canister
}

main() {

    dfx stop || killall dfx icx-proxy || true

    dfx start --clean --background
    # Wait for dfx started
    sleep 5

    deploy_iceth_client_test_canisters
    deploy_ic_sign_test_canister
}

main "$@"
