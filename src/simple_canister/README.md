# Simple Canister

This example is used to set up a minimal canister that registers on the EVM testnet

## Background

For more information about `What is the Bitfinity EVM` and more, please see [docs](https://tech-docs-evmc.vercel.app/).

As described in the [docs](https://tech-docs-evmc.vercel.app/ic-agent/overview):
> To use certain EVM canister features, such as `call_message`, `create_contract`, and `withdraw_tokens`, users must register an EVM address for their IC principal. 

Currently EVM canister supports two ways to receive transactions. One is to use Ethereum tools (such as `Metamask`) to sign the transaction and send it to the EVM canister through JSONPRC. The other is to use the IC tool to call the canister endpoint of EVM canister, if so, EVM canister can only obtain the Principal from msg.caller, but cannot obtain the Ethereum address and signature.
Therefore, in order to eliminate the incompatibility between EVM and IC principal, users must register an EVM address for their IC principal.

## How IC agent works
We will bind an Ethereum address to a principal step by step according to [this doc](https://tech-docs-evmc.vercel.app/ic-agent/overview)

### Get self principal
```sh
dfx identity get-principal
yhy6j-huy54-mkzda-m26hc-yklb3-dzz4l-i2ykq-kr7tx-dhxyf-v2c2g-tae
```
### Generate private key & signature
The [signature package](../signature/src/main.rs) will help us generate the required signature.

Because the whole process requires us to send an Ethereum private key (signing key) to EVM canister, so there needs to be a [security statement](https://tech-docs-evmc.vercel.app/ic-agent/overview#verify-registration) about this:
> After step 3 & 4, other IC canisters will not be able to use the registered address when creating transactions, making it safe to expose the signing key at this point.

Also, this signing key should be one-time, i.e. it cannot be used for other Ethereum compatible projects either.

So, Let's start:
```sh
cd register-evm-agent

cargo run --bin signature
private key: [81, 72, 69, 68, 94, 35, 255, 67, 238, 77, 189, 96, 235, 181, 172, 162, 60, 166, 12, 240, 207, 30, 28, 188, 136, 11, 249, 108, 197, 123, 241, 190]
r: 0xdbb3af3eda0d65ff1e71dcd720a14bde8f4daeda54b2910c7bb32f26ed53d02c, s: 0x1cd0c88b0feb607772c9d59fe716fbb29d920238baeda4786e0191fc44e0c57a, v: 0xad676
tx hash: 0x41b56fadd83a943582c91c62411f9e302d36c177dd8ba18ff257f1750d678a93
tx: Legacy(TransactionRequest { from: Some(0x20bc9e20dfef83780349356779b9b688552ccbb0), to: Some(Address(0xb0e5863d0ddf7e105e409fee0ecc0123a362e14b)), gas: Some(21000), gas_price: Some(10), value: Some(100000), data: None, nonce: Some(0), chain_id: Some(355113) })
```
signature package will generate a private key randomly each time, so it only needs to be run once.

According to the log out, we get:   
from address: `0x20bc9e20dfef83780349356779b9b688552ccbb0`   
to address: `0xb0e5863d0ddf7e105e409fee0ecc0123a362e14b`   
gas: $21000_{10} = 5208_{16}$   
gasPrice: $10_{10} = a_{16}$   
value: $100000_{10} = 186a0_{16}$   
chainId: $355113_{10} = 56b29_{16}$
and signature's r, s, v and tx's hash.

Obviously, this address should not be registered, but let's double check: 
```sh
dfx canister --network ic call evmc is_address_registered '("0x20bc9e20dfef83780349356779b9b688552ccbb0")' --query
(false)
```

### recharge to the new address

Recharge 1_600_000(0x186a00) evm naive token to the from address so that it can pay the gas fee in the future:
```sh
dfx canister --network ic call evmc mint_evm_tokens '("0x20bc9e20dfef83780349356779b9b688552ccbb0", "0x186a00")'
(variant { Ok = "0x186a00" })

dfx canister --network ic call evmc account_basic '("0x20bc9e20dfef83780349356779b9b688552ccbb0")' --query
(record { balance = "0x186a00"; nonce = "0x0" })
```

### call register_ic_agent

Use data from above:
```sh
dfx canister --network ic call evmc register_ic_agent '(record {r="0xdbb3af3eda0d65ff1e71dcd720a14bde8f4daeda54b2910c7bb32f26ed53d02c";s="0x1cd0c88b0feb607772c9d59fe716fbb29d920238baeda4786e0191fc44e0c57a";v="0xad676";to=opt "0xb0e5863d0ddf7e105e409fee0ecc0123a362e14b";gas="0x5208";maxFeePerGas=null;gasPrice=opt "0xa";value="0x186a0";blockNumber=null;from="0x20bc9e20dfef83780349356779b9b688552ccbb0";hash="0x41b56fadd83a943582c91c62411f9e302d36c177dd8ba18ff257f1750d678a93";blockHash=null;"type"=null;accessList=null;transactionIndex=null;nonce="0x0";maxPriorityFeePerGas=null;input="";chainId=opt "0x56b29"})'
(variant { Ok })
```
Success!

### call verify_registration

Use data from above:
```sh
dfx canister --network ic call evmc verify_registration '(vec {81:nat8;72:nat8;69:nat8;68:nat8;94:nat8;35:nat8;255:nat8;67:nat8;238:nat8;77:nat8;189:nat8;96:nat8;235:nat8;181:nat8;172:nat8;162:nat8;60:nat8;166:nat8;12:nat8;240:nat8;207:nat8;30:nat8;28:nat8;188:nat8;136:nat8;11:nat8;249:nat8;108:nat8;197:nat8;123:nat8;241:nat8;190:nat8})'
(variant { Ok })
```
Success, let's check that:
```sh
dfx canister --network ic call evmc is_address_registered '("0x20bc9e20dfef83780349356779b9b688552ccbb0")' --query
(true)

dfx canister --network ic call evmc account_basic '("0x20bc9e20dfef83780349356779b9b688552ccbb0")' --query
(record { balance = "0x13af10"; nonce = "0x1" })
```

### make a transfer from principal

Now, my principal `yhy6j-huy54-mkzda-m26hc-yklb3-dzz4l-i2ykq-kr7tx-dhxyf-v2c2g-tae` can use the balance under this address `0x20bc9e20dfef83780349356779b9b688552ccbb0`

Let's send 255(0xff) token to `0x000000000000000000000000000000000000dEaD`:
```sh
dfx canister --network ic call evmc call_message '(record {value="0xff";from="0x20bc9e20dfef83780349356779b9b688552ccbb0";nonce="0x1";gas_limit=21000:nat64;gas_price=null;}, "0x000000000000000000000000000000000000dEaD", "")'
(
  variant {
    Ok = "0x0cc36a9e3aee62f2b36d8380baa3c95ecb6bc068ad1e6fc3fb58ad3a3dda58d4"
  },
)
```
Success, we can see result in [explorer](https://explorer.bitfinity.network/tx/0x0cc36a9e3aee62f2b36d8380baa3c95ecb6bc068ad1e6fc3fb58ad3a3dda58d4)

and the account state of `0x20bc9e20dfef83780349356779b9b688552ccbb0` changed.
```sh
dfx canister --network ic call evmc account_basic '("0x20bc9e20dfef83780349356779b9b688552ccbb0")' --query
(record { balance = "0x1079c1"; nonce = "0x2" })
```

## Build

Install [ic-wasm](https://github.com/dfinity/ic-wasm) first, and run:
```sh
cargo run -p simple_canister --features "export-api" > ./.artifact/simple_canister.did

cargo build --target wasm32-unknown-unknown --release --package simple_canister --features "export-api"

ic-wasm target/wasm32-unknown-unknown/release/simple_canister.wasm -o ./.artifact/simple_canister.wasm shrink

```

## registy