#!/bin/bash

## exit if something fails
set -e

START_BLOCK_NUMBER=19024188
END_BLOCK_NUMBER=19025187

for i in $(seq $START_BLOCK_NUMBER $END_BLOCK_NUMBER); do
    printf -v hex_number %x $i
    echo $hex_number

    LINE_SEPARATOR='--------------------------------------------------------'

    echo $LINE_SEPARATOR
    echo 'Fetching block [' $hex_number '] from Ethereum'

    JSON_DATA="{\"jsonrpc\":\"2.0\",\"method\":\"eth_getBlockByNumber\",\"params\":[\"0x$hex_number\",false],\"id\":1}"

    # [optional] validate the string is valid json
    echo $JSON_DATA | jq

    curl -X POST --data $JSON_DATA https://cloudflare-eth.com/ -v | jq . > ./block/$i.json

    echo 'Done'
    echo $LINE_SEPARATOR

done
