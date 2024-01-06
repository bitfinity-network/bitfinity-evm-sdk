# EVM Block Extractor And Server

## EVM Block Extractor

### Introduction

The EVM block extractor is an advanced tool used to collect EVM blocks and transactions, and send them to a specified BigQuery dataset endpoint. This version is enhanced to handle parallel requests efficiently and integrates with Google Cloud Platform's BigQuery service.

### Usage

```sh
evm-block-extractor
  --rpc-url <evmc-rpc-url>
  --dataset-id <bigquery-dataset-id>
  --max-number-of-requests <max-parallel-requests>
  --rpc-batch-size <rpc-batch-size>
  --sa-key <service-account-key>
```

Where:

- **rpc-url**: is the endpoint of the EVMC json-rpc url
- **dataset-id**: is the BigQuery dataset id where the data will be sent
- **max-number-of-requests**: is the maximum number of parallel requests to be sent to the EVMC json-rpc endpoint
- **rpc-batch-size**: is the number of blocks to be requested in a single batch
- **sa-key**: the service account key in JSON format for GCP authentication.

### Output

The data is sent and stored in the specified BigQuery dataset. This allows for enhanced querying and analysis capabilities using BigQuery's features.

## EVM Block Extractor Server

### Introduction

The EVM block extractor server is a JSON-RPC server for the EVM block extractor. It is integrated with BigQuery and allows for querying the data stored in the BigQuery dataset.

### Usage

```sh
evm-block-extractor-server
  --dataset-id <bigquery-dataset-id>
  --server-address <server-address>
  --sa-key <service-account-key>
```

Where:

- **dataset-id**: The dataset ID of the BigQuery table.
- **server-address:** The address where the server will be hosted (default: 127.0.0.1:8080).
- **sa-key**: The service account key in JSON format for GCP authentication.

### Endpoints

This is minimal version of the Ethereum JSON-RPC server. It supports the following endpoints:

- **eth_blockNumber**: Returns the number of most recent block.
- **eth_getBlockByNumber**: Returns information about a block by block number.
- **eth_getTransactionReceipt**: Returns the receipt of a transaction by transaction hash.
- **ic_getBlocksRLP**: Returns a list of blocks in RLP format.

### Example

```sh
curl -X POST --data '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' http://127.0.0.1:8080
```
