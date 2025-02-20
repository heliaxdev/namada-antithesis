#!/bin/bash

# Wait for the JSON RPC to come up for validator 2
json_rpc_ready=0
while [ $json_rpc_ready != 200 ]
do
    echo "Checking node rpc ${RPC}/status ..."
    json_rpc_ready=$(curl -s -o /dev/null -w "%{http_code}" "http://${RPC}/status")
    echo "Node rpc query result: $json_rpc_ready"
    sleep 2
done

# Finding the CHAIN ID from the common volume mount directory
CHAIN_ID=$(find /container_ready -type f -name "devnet*")
while [[ -z $CHAIN_ID ]]
do
    echo Waiting for the chain ID
    CHAIN_ID=$(find /container_ready -type f -name "devnet*")
    sleep 2
done

CHAIN_ID=$(basename $CHAIN_ID)

# Wait for the JSON RPC to come up for masp indexer
json_rpc_ready=0
while [ $json_rpc_ready != 200 ]
do
    echo "Checking masp indexer ${MASP_INDEXER_URL}/health ..."
    json_rpc_ready=$(curl -s -o /dev/null -w "%{http_code}" "${MASP_INDEXER_URL}/health")
    echo "Masp indexer query result: $json_rpc_ready"
    sleep 2
done

source /opt/antithesis/test/v1/namada/parallel_driver_create_wallet.sh
source /opt/antithesis/test/v1/namada/parallel_driver_create_wallet.sh
source /opt/antithesis/test/v1/namada/parallel_driver_create_wallet.sh
source /opt/antithesis/test/v1/namada/parallel_driver_create_wallet.sh
source /opt/antithesis/test/v1/namada/parallel_driver_create_wallet.sh
source /opt/antithesis/test/v1/namada/parallel_driver_create_wallet.sh
source /opt/antithesis/test/v1/namada/parallel_driver_create_wallet.sh
source /opt/antithesis/test/v1/namada/parallel_driver_create_wallet.sh
source /opt/antithesis/test/v1/namada/parallel_driver_create_wallet.sh
source /opt/antithesis/test/v1/namada/parallel_driver_create_wallet.sh

source /opt/antithesis/test/v1/namada/parallel_driver_faucet_transfer.sh
source /opt/antithesis/test/v1/namada/parallel_driver_faucet_transfer.sh
source /opt/antithesis/test/v1/namada/parallel_driver_faucet_transfer.sh
source /opt/antithesis/test/v1/namada/parallel_driver_faucet_transfer.sh
source /opt/antithesis/test/v1/namada/parallel_driver_faucet_transfer.sh
source /opt/antithesis/test/v1/namada/parallel_driver_faucet_transfer.sh
source /opt/antithesis/test/v1/namada/parallel_driver_faucet_transfer.sh
source /opt/antithesis/test/v1/namada/parallel_driver_faucet_transfer.sh
source /opt/antithesis/test/v1/namada/parallel_driver_faucet_transfer.sh
source /opt/antithesis/test/v1/namada/parallel_driver_faucet_transfer.sh
source /opt/antithesis/test/v1/namada/parallel_driver_faucet_transfer.sh
source /opt/antithesis/test/v1/namada/parallel_driver_faucet_transfer.sh
source /opt/antithesis/test/v1/namada/parallel_driver_faucet_transfer.sh
source /opt/antithesis/test/v1/namada/parallel_driver_faucet_transfer.sh

source /opt/antithesis/test/v1/namada/parallel_driver_bond.sh
source /opt/antithesis/test/v1/namada/parallel_driver_bond.sh
source /opt/antithesis/test/v1/namada/parallel_driver_bond.sh
source /opt/antithesis/test/v1/namada/parallel_driver_bond.sh
source /opt/antithesis/test/v1/namada/parallel_driver_bond.sh
source /opt/antithesis/test/v1/namada/parallel_driver_bond.sh
source /opt/antithesis/test/v1/namada/parallel_driver_bond.sh
source /opt/antithesis/test/v1/namada/parallel_driver_bond.sh
source /opt/antithesis/test/v1/namada/parallel_driver_bond.sh
source /opt/antithesis/test/v1/namada/parallel_driver_bond.sh
source /opt/antithesis/test/v1/namada/parallel_driver_bond.sh
source /opt/antithesis/test/v1/namada/parallel_driver_bond.sh
source /opt/antithesis/test/v1/namada/parallel_driver_bond.sh
source /opt/antithesis/test/v1/namada/parallel_driver_bond.sh
source /opt/antithesis/test/v1/namada/parallel_driver_bond.sh

source /opt/antithesis/test/v1/namada/parallel_driver_init_account.sh
source /opt/antithesis/test/v1/namada/parallel_driver_init_account.sh
source /opt/antithesis/test/v1/namada/parallel_driver_init_account.sh
source /opt/antithesis/test/v1/namada/parallel_driver_init_account.sh
source /opt/antithesis/test/v1/namada/parallel_driver_init_account.sh

# Ready to start workload
echo "Ready to start the workload"
