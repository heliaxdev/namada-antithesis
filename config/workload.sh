#!/bin/bash

set -e

VALIDATOR_RPC=${VALIDATOR_RPC:-"30.0.0.12:27658"}
FAUCET_SK=${FAUCET_SK:-"00dfd790bd727b708f8b846374c596d886eaf1ebf0fc4394530e0a9b24aa630963"}

# Wait for the JSON RPC to come up for validator 0
json_rpc_ready=0
while [ $json_rpc_ready -eq 0 ]
do
    json_rpc_ready=$(curl -I ${VALIDATOR_RPC}/status | grep 200 | wc -l)
    # echo "Checking validator JSON RPC status on $VALIDATOR_RPC/status"
    # response=$(curl -X POST ${VALIDATOR_RPC}/status)
    # health_check=$(echo $response | jq '.result.sync_info.earliest_block_height')
    # if [ $health_check = '"1"' ]; then
    #     echo JSON RPC is healthy, workload ready to start!
    #     json_rpc_ready=1
    # fi
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

# Ready to start workload
echo "Ready to start the workload"

./namada-chain-workload --rpc http://${VALIDATOR_RPC} --chain-id ${CHAIN_ID} --faucet-sk ${FAUCET_SK}
