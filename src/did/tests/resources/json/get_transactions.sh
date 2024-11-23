#!/bin/bash

## exit if something fails
set -e

##
## This script downloads from the Ethereum network the transactions in the hash_list
## and saves each of them in a json file named TRANSACTION_HASH.json
##

declare -a hash_list=(
    # Type 0x0
    "0xe1ffa2abdc95ebfa92d3178698c4feea7615d3669d16bf5929924881893837ce"
    "0x20081e3012905d97961c2f1a18e1f3fe39f72a46b24e078df2fe446051366dca"
    # Type 0x2
    "0xdcede6a4ac8829a7f18d3994e9a0e30d913e7d5b4cdb1106aafd9b0118d405a3"
    "0x1f336059dde3447fe37e3969a50857597515c753e8336b7e406792a4176bd60f"
    "0xd5f627e3ad2e6e0f4a131c52142e2a8344cd9077965c2404fa4ec555113b4ca6"
    "0x945ed16321825a4610de3ecc51b2920659f390c5ee96ac468f57ee56aab45ff9"
)

for i in "${hash_list[@]}"
do
    LINE_SEPARATOR='--------------------------------------------------------'

    echo $LINE_SEPARATOR
    echo 'Fetching transaction [' $i '] from Ethereum'

    JSON_DATA="{\"jsonrpc\":\"2.0\",\"method\":\"eth_getTransactionByHash\",\"params\":[\"$i\"],\"id\":1}"

    # [optional] validate the string is valid json
    echo $JSON_DATA | jq

    curl -X POST --data $JSON_DATA https://cloudflare-eth.com/ -v | jq . > ./transaction/$i.json

    echo 'Done'
    echo $LINE_SEPARATOR

done

