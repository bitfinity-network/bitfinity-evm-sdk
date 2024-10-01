
# Cleans the build artifacts
[group('build')]
[confirm("Are you sure you want to clean the build artifacts?")]
clean:
  rm -rf {{WASM_DIR}}
  cargo clean


# Builds the test canister
[group('build')]
build:
  mkdir -p {{WASM_DIR}}
  echo "Building ic-sign-client test canisters"
  cargo run -p ic-sign-test-canister --features "export-api" > {{WASM_DIR}}/ic-sign-test-canister.did
  cargo build -p ic-sign-test-canister --target wasm32-unknown-unknown --release --features "export-api"
  ic-wasm target/wasm32-unknown-unknown/release/ic-sign-test-canister.wasm -o {{WASM_DIR}}/ic-sign-test-canister.wasm shrink
  gzip -k "{{WASM_DIR}}/ic-sign-test-canister.wasm" --force
