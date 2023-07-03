#!/bin/bash

export WASM_DIR=.artifact
mkdir -p $WASM_DIR

# echo "Building iceth-client artifacts..."
# ./src/iceth-client/scripts/download_iceth.sh
# ./src/iceth-client/scripts/build_test_canister.sh

echo "Building eth-signer artifacts..."
./src/eth-signer/scripts/build_test_canister.sh