#!/bin/bash

set -e

# Wait for the JSON RPC to come up for validator 0
json_rpc_ready=0
while [ $json_rpc_ready -eq 0 ]
do
    json_rpc_ready=$(curl -I ${RPC}/status | grep 200 | wc -l)
    sleep 10
done

# Finding the CHAIN ID from the common volume mount directory
CHAIN_ID=$(find /container_ready -type f -name "devnet*")
while [[ -z $CHAIN_ID ]]
do
    echo Waiting for the chain ID
    CHAIN_ID=$(find /container_ready -type f -name "devnet*")
    sleep 10
done

CHAIN_ID=$(basename $CHAIN_ID)

echo "Workload: the chain ID is $CHAIN_ID"
echo "Using rpc: ${RPC}"

# Ready to start workload
echo "Ready to start the workload"