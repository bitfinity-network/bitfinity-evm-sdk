#!/bin/bash

dfx deploy iceth 
ICETH_PRINCIPAL=$(dfx canister id iceth)

INIT_ARGS="record { url=\"https://testnet.bitfinity.network\"; iceth=principal \"$ICETH_PRINCIPAL\"; chain_id=355113}"
dfx deploy iceth-client-test-canister --argument "$INIT_ARGS" -m reinstall -y
