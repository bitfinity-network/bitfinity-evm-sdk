#!/bin/bash
set -e

echo "Starting entrypoint.sh"

/app/evm-block-extractor-server -s 0.0.0.0:8080 --bigquery -d $DATASET -p $PROJECT_ID
