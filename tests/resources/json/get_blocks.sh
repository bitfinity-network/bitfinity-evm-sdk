#!/bin/bash

## exit if something fails
set -e

##
## This script downloads from the Ethereum network the blocks in the hash_list
## and saves each of them in a json file named TRANSACTION_HASH.json
##

declare -a hash_list=(
    "0xb2f703a57637e49572b16088b344db8fb108246f8360027ca8831766443a9c02"
    "0x81f6e266d34db0c21165d78e0c5e37dab36aee204d0a4422533200fcc8a37b93"
    "0x9ee9da5fafb45610f3c2ba78abe34bd46be01f4de29fc2704a81a76c8171038e"
    "0x207dc8087bbdbef42146c9c31f5df79266c1c61be209416abf7d5ed260a63a21"
    "0xecea9251184f99ea1b65927b665363dd22d5fcf08f350e4157063fd34175d111"
)

for i in "${hash_list[@]}"
do
    LINE_SEPARATOR='--------------------------------------------------------'

    echo $LINE_SEPARATOR
    echo 'Fetching block [' $i '] from Ethereum'

    JSON_DATA="{\"jsonrpc\":\"2.0\",\"method\":\"eth_getBlockByHash\",\"params\":[\"$i\",false],\"id\":1}"

    # [optional] validate the string is valid json
    echo $JSON_DATA | jq

    curl -X POST --data $JSON_DATA https://cloudflare-eth.com/ -v | jq . > ./block/$i.json

    echo 'Done'
    echo $LINE_SEPARATOR

done
