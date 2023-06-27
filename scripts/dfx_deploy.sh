#!/bin/bash

export WASM_DIR=.artifact

dfx start --clean --background

# Wait for dfx started
sleep 5

echo "Deploying iceth-client artifacts..."
./src/iceth-client/scripts/dfx_deploy.sh

echo "Deploying eth-signer artifacts..."
dfx deploy ic-sign-test-canister


