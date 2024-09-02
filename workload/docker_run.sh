#!/bin/bash

set -e

./namada-chain-workload --rpc ${RPC} --faucet-sk ${FAUCET_SK} --chain-id ${CHAIN_ID}