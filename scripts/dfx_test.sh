#!/bin/bash

export WASM_DIR=./target/artifact

echo "Running dfx tests for iceth-client..."
dfx canister call iceth-client-test-canister test_send_raw_transaction_signed_with_signing_key
dfx canister call iceth-client-test-canister test_send_raw_transaction_signed_with_management_canister

echo "Running dfx tests for eth-signer..."
dfx canister call ic-sign-test-canister sign_and_check