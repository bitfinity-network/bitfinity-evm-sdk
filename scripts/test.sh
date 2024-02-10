#!/usr/bin/env sh
set -e
set -x #echo on

export RUST_BACKTRACE=full

# before testing, the build.sh script should be executed
cargo test test_insert_and_fetch_last_block_certified_data
cargo test $@ --all-features
