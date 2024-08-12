#!/usr/bin/env sh
set -e
set -x #echo on

export RUST_BACKTRACE=full

if [ -z "$ALCHEMY_API_KEY" ]; then
    echo "ALCHEMY_API_KEY is not set"
    exit 1
fi

# before testing, the build.sh script should be executed
cargo test $@
cargo test $@ --all-features
