## Register Minter Cli

A Cli tool for generating an ETH Wallet & registering minter canister to the evmc

### Build

Run below to build the binary.

```sh
./scripts/build.sh
```

The binary can be found at the path `.artifacts/register_minter`.

### Usage

For general information how to use the cli, run below

```sh
register_minter --help
```

Run below to get help information on the various subcommands

```sh
register_minter help generate-wallet
register_minter help register
```

### Examples

To generate only a wallet, run below:

```sh
register_minter generate-wallet
```

To generate new wallet & register minter canister to evmc, run below:

```sh
register_minter register <path_to_identity_pem_file> <evmc_canister_id> <minter_canister_id>
```

To register minter canister with an existing wallet

```sh
register_minter register -k <wallet_private_key_hex> <path_to_identity_pem_file> <evmc_canister_id> <minter_canister_id>
```
