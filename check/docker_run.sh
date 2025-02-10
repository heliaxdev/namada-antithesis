#!/bin/bash

set -e

RPC=${RPC:-"30.0.0.14:27658"}

echo "Using rpc: ${RPC}"
echo "Using masp indexer url: ${MASP_INDEXER_URL}"

./namada-chain-check --rpc http://${RPC} --masp-indexer-url ${MASP_INDEXER_URL}