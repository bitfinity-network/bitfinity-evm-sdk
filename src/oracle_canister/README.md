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
dfx canister call oracle_canister add_pair '("bitcoin")'
dfx canister call oracle_canister add_pair '("ethereum")'
dfx canister call oracle_canister add_pair '("internet-computer")'
dfx canister call oracle_canister add_pair '("ordinals")'
dfx canister call oracle_canister add_pair '("dfuk")'
dfx canister call oracle_canister add_pair '("pepebrc")'
dfx canister call oracle_canister add_pair '("pizabrc")'
dfx canister call oracle_canister add_pair '("biso")'
dfx canister call oracle_canister add_pair '("meme-brc-20")'
```

Open link: `http://127.0.0.1:8000/?canisterId=<Oracle_Canister_Id>` such as `http://127.0.0.1:8000/?canisterId=bnz7o-iuaaa-aaaaa-qaaaa-cai` in browser. 
