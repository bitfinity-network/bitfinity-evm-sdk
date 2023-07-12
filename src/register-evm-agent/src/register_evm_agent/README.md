# Register EVM Agent

A Cli tool for generating an ETH Wallet & reserving a canister to the EVMC

## Build

Run the commands below to install the binary.

```sh
cargo install register-evm-agent
```

## Usage

For general information how to use the cli, run below

```sh
register-evm-agent --help
```

### Generate wallet

If you need to generate a wallet first, you can run

```sh
register-evm-agent generate-wallet
```

The command output will display the generated wallet info like this:

```txt
Wallet:
  Private Key = 048f4682aa84d9c92f4452956896e459a5d8b675895ca0a7dca6028641256c12
  Public Key = 0219aa742ea1020079d2f503d754db1d0c76e240ab2c43bcf71ab1ca91a099c13b
  Address = 0x6d4662d3ab4769a4f10781325601db68874261d2
```

### Reserve canister

To reserve a canister the following command needs to be run

```sh
register-evm-agent reserve -k <private_key> -n <network> -i <identity_path> --evmc <evmc_principal> --canister-id <reserve_canister_principal>
```

Where:

- `private key` is the Private key for the generated wallet
- `network` is the network to run against: default is `local`, the value can be both `ic` or a custom URL.
- `identity path` is the path to the identity you're going to use to reserve your canister
- `evmc principal` is the principal for the EVMC canister
- `reserve canister principal` is the principal of the canister you're going to reserve

All the supported options can be seen with

```sh
register-evm-agent reserve --help
```

#### Additional options

- **Amount to mint**: if you're using a testnet and you need to mint native tokens to your wallet first, you can pass the amount of tokens you need to mint to your wallet before reserving the canister

    ```sh
    register-evm-agent reserve -k ... -a 1000000000 ...
    ```

- **Specify the chain id**: you can specify the cain id providing the id as an argument

    ```sh
    register-evm-agent reserve -k ... -C <custom-chain-id>
    ```
