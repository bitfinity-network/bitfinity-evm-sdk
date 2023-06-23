#!/bin/sh

dfx deploy test_canister
SIGN_RESULT=$(dfx canister call test_canister sign_and_check)

if [ "$SIGN_RESULT" != "()" ]; 
then
    echo "Unexpected result: $SIGN_RESULT" >&2
    exit 1
else
    echo "Signing works correctly"
fi
