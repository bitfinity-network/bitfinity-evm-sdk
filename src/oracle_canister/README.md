# Oracle canister

## build
```sh
cargo run -p oracle_canister --features "export-api" > .artifact/oracle_canister.did

cargo build --target wasm32-unknown-unknown --release --package oracle_canister --features "export-api"

ic-wasm target/wasm32-unknown-unknown/release/oracle_canister.wasm -o .artifact/oracle_canister.wasm shrink
```

## deploy local
terminal 0:
```sh
dfx start --clean
```

terminal 1:
```sh
dfx canister create --no-wallet oracle_canister

dfx build oracle_canister

dfx canister install oracle_canister --argument "record { evmc_principal=principal \"aaaaa-aa\";owner=principal \"$(dfx identity get-principal)\"}"

# add cryptocurrency pairs
dfx canister call oracle_canister add_pair '("ETH-USD")'
dfx canister call oracle_canister add_pair '("BTC-USD")'
dfx canister call oracle_canister add_pair '("ICP-USD")'
dfx canister call oracle_canister add_pair '("SHIB-USD")'
```

Open link: `http://127.0.0.1:8000/?canisterId=<Oracle_Canister_Id>` such as `http://127.0.0.1:8000/?canisterId=bnz7o-iuaaa-aaaaa-qaaaa-cai` in browser. 
