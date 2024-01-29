#!/bin/bash
set -e
set -x #echo on

export RUST_BACKTRACE=full

test_ic_sign_test_canister() {
    echo "Running dfx tests for eth-signer..."
    dfx canister call ic-sign-test-canister sign_and_check
    dfx canister call ic-sign-test-canister non_deterministic_signing
}


main() {

    ./scripts/dfx_deploy.sh

    test_ic_sign_test_canister
}

main "$@"
