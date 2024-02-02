# EVM Block Extractor

## Introduction

The EVM block extractor is an advanced tool used to collect EVM blocks and transactions, and send them to a specified data storage. 
This version is enhanced to handle parallel requests efficiently and integrates with Google Cloud Platform's BigQuery service or Postgres.

## Configuration

### Usage with BigQuery

```sh
evm-block-extractor
  --server-address <server-address>
  --rpc-url <evmc-rpc-url>
  --max-number-of-requests <max-parallel-requests>
  --rpc-batch-size <rpc-batch-size>
  --bigquery
  --project-id <bigquery-project-id>
  --dataset-id <bigquery-dataset-id>
  --sa-key <service-account-key>
```

Where:

- **server-address:** The address where the server will be hosted (default: 127.0.0.1:8080).
- **rpc-url**: is the endpoint of the EVMC json-rpc url
- **max-number-of-requests**: is the maximum number of parallel requests to be sent to the EVMC json-rpc endpoint
- **rpc-batch-size**: is the number of blocks to be requested in a single batch
- **dataset-id**: is the BigQuery dataset id where the data will be sent
- **sa-key**: the service account key in JSON format for GCP authentication.


### Usage with Postgres

```sh
evm-block-extractor
  --server-address <server-address>
  --rpc-url <evmc-rpc-url>
  --max-number-of-requests <max-parallel-requests>
  --rpc-batch-size <rpc-batch-size>
  --postgres
  --username <postgres-db-username>
  --password <postgres-db-password>
  --database_name <postgres-db-name>
  --database_url <postgres-db-url>
  --database_port <postgres-db-port>
  --require_ssl <postgres-db-require-ssl>
```

Where:

- **username**: Username for the database connection
- **password**: Password for the database connection
- **database_name**: database name
- **database_url**: database IP or URL
- **database_port**: database port
- **require_ssl**: whether to use ssl (true/false)


## Endpoints

Th evm-block-extracor is also a minimal version of the Ethereum JSON-RPC server which supports the following endpoints:

- **eth_blockNumber**: Returns the number of most recent block.
- **eth_getBlockByNumber**: Returns information about a block by block number.
- **eth_getTransactionReceipt**: Returns the receipt of a transaction by transaction hash.
- **ic_getBlocksRLP**: Returns a list of blocks in RLP format.

### Example

```sh
curl -X POST -H "Content-Type: application/json" --data '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' http://127.0.0.1:8080
```

## Docker image

The evm-block-extractor docker image is a debian slim based image that allows for simple installation of the service.
The docker image accepts the same configuration arguments of the plain executor. 
E.g.:
```sh
docker run ghcr.io/bitfinity-network/evm-block-extractor:main --rpc-url https://testnet.bitfinity.network --postgres --username postgres --password postgres --database-name postgres --database-url 127.0.0.1:5432
```

