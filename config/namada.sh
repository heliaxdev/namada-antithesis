#!/bin/bash
 
# Simple entrypoint for the validator/full node to wait for joining the network before starting
set -e

BASE_DIR=${BASE_DIR:-"/validator0"}

joined_network=0
while [ $joined_network -eq 0 ]
do
    node=$(echo $BASE_DIR | tr -d "/")
    echo "Checking if $node has joined the network"
    if [ -e "/container_ready${BASE_DIR}" ]; then
        echo "$node ready to start"
        joined_network=1
    fi
    sleep 10
done

/namada/target/release/namadan ledger run --base-dir $BASE_DIR
