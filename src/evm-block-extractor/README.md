# EVM Block Extractor

## Introduction

The EVM block extractor is a tool which can be used to collect blocks and block receipts from the JSON-RPC EVMC endpoint into a ZIP file.

## Usage

```sh
evm-block-extractor
  --rpc-url <evmc-rpc-url>
  --output-file <output-zipfile>
  --start-block <start-block>
  --end-block <last-block>
  --batch-size <max-batch-size>
```

Where:

- **rpc-url**: is the endpoint of the EVMC json-rpc url
- **output-file**: path to the ZIP file to write blocks to
- **start-block**: the number of the block to start collecting blocks from
- **end-block**: the number of the last block to fetch. If not provided blocks will be collected until the last one is reached.
- **batch-size**: maximum amount of blocks to collect from a single request. (default: 500)

## Output file

The output file is a `zip` file containing a receipt and a block file for each block.
The file names syntax is the following one:

- `block_0x{block_number}.json`: JSON encoded block identified by its number in hex representation with 16 digits
- `receipt_0x{block_number}.json`: JSON encoded receipt identified by its number in hex representation with 16 digits

## Resuming

If the specified `output-file` argument already exists, the extractor will try to read the last archived block number and will start from there.
