#!/bin/bash

TAG=v0.1.1
ICETH_URL=https://github.com/infinity-swap/iceth/releases/download/${TAG}/iceth-${TAG}.tar.gz

echo "Downloading $ICETH_URL"
curl -fsSL $ICETH_URL | tar -xz -C "$WASM_DIR"