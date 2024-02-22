# EVM Block Extractor

## Introduction

The EVM log extractor is a tool used to collect logs from the EVM canister. 

## Configuration

To run the log extractor use CLI command:
```bash
evm-log-extractor [OPTIONS]
```

### Requirements

The principal utilizing this tool must have the `ReadLogs` permission configured for the EVMC canister instance from which logs are to be downloaded. This permission should be granted by an administrator using the following command: 

```bash
dfx canister call <EVMC_CANISTER_ID> admin_ic_permissions_add '(principal "<LOG_EXTRACTOR_PRINCIPAL_ID>", vec {variant { ReadLogs }})' --network ic
```

### CLI options

- `--logger-filter <LOGGER_FILTER>`

Sets the logger `EnvFilter`. Valid values: `trace`, `debug`, `info`, `warn`, `error`. Example of a valid filter: `warn,my_crate=info,my_crate::my_mod=debug,[my_span]=trace`. Default: `info`.

- `--evmc-network-url <REMOTE_URL>`

URL of the EVMC network.
Default: http://127.0.0.1:8000

- `--identity <PATH_TO_IC_IDENTITY_PEM_FILE>`

Path to your identity pem file.

- `--evmc-principal <EVMC_PRINCIPAL>`

Evmc canister Principal.

- `--logs-synchronization-job-interval-seconds <SECONDS>`

Logs synchronization job interval in seconds.
This job executes is executed every <logs_synchronization_job_interval_seconds> seconds and download the 
evmc logs to a file on the local filesystem. The job is enables only if both `identity` and `evmc_principal` are provided.
Default is 10 seconds.

- `--logs-synchronization-job-max-logs-per-call <MAX_LOGS_PER_CALL>`

The max number of logs to be downloaded on each log synchronization job loop.
Default is 5_000.

- `--logs-directory <LOGS_DIRECTORY>`

Path to the directory where the EVM downloaded logs are written into.


### Examples

Example of how to run the log extractor from source code targeting an evmc canister running in a local dfx replica:

`$ cargo run -p evm-log-extractor -- --evmc-principal=bkyz2-fmaaa-aaaaa-qaaaq-cai --identity ~/.config/dfx/identity/alice/identity.pem --logs-directory ./target/logs --evmc-network-url "http://127.0.0.1:38985"`


## Docker image

The evm-log-extractor docker image is an ubuntu:22.04 based image that allows for simple installation of the service.

The docker image accepts the following configuration variables: 

- `LOGGER_FILTER`: (Optional) the level of the logger. Default is `info`
- `EVMC_PRINCIPAL`: (Mandatory) the canister ID of the evmc canister
- `EVMC_NETWORK_URL`: (Optional) the URL of the IC network. Default is `https://icp0.io`

It is also required to configure these volumes:
- `/data/config/identity.pem`: mount point for the identity pem file to be used for calling the evmc canister
- `/data/logs`: mount point where the extracted logs will be persisted

E.g.:
```sh
docker run ghcr.io/bitfinity-network/evm-log-extractor:main \
    -e EVMC_PRINCIPAL=bkyz2-fmaaa-aaaaa-qaaaq-cai \
    -v ~/.config/dfx/identity/alice/identity.pem:/data/config/identity.pem:ro \
    -v ./target/logs:/data/logs
```
