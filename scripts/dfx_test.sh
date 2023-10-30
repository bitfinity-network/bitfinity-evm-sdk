#!/bin/bash
set -e
export RUST_BACKTRACE=full

test_iceth_client_test_canisters() {
    echo "Running dfx tests for iceth-client..."
    dfx canister call iceth-client-test-canister test_send_raw_transaction_signed_with_signing_key
    dfx canister call iceth-client-test-canister test_send_raw_transaction_signed_with_management_canister
}

test_ic_sign_test_canister() {
    echo "Running dfx tests for eth-signer..."
    dfx canister call ic-sign-test-canister sign_and_check
    dfx canister call ic-sign-test-canister non_deterministic_signing
}


main() {

    ./scripts/dfx_deploy.sh

    test_iceth_client_test_canisters
    test_ic_sign_test_canister
}

main "$@"
