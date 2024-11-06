#!/bin/bash

set -e

/app/namada-chain-workload --rpc http://${RPC} --chain-id ${CHAIN_ID} --faucet-sk ${FAUCET_SK} --id ${WORKLOAD_ID} --masp-indexer-url ${MASP_INDEXER_URL} new-wallet-key-pair