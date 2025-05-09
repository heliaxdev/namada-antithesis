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

# Wait for IBC channel
while [ ! -f /container_ready/ibc_channels ]
do
    echo "Waiting for IBC channels to be created..."
    sleep 5
done

namada_channel_id=$(grep "namada->cosmos" /container_ready/ibc_channels | grep -o "channel-[0-9]\+")
cosmos_channel_id=$(grep "cosmos->namada" /container_ready/ibc_channels | grep -o "channel-[0-9]\+")

echo "Creating config.toml..."
cat <<EOF > config.toml
id = ${WORKLOAD_ID}
chain_id = "${CHAIN_ID}"
rpc = "http://${RPC}"
masp_indexer_url = "${MASP_INDEXER_URL}"
faucet_sk = "${FAUCET_SK}"
cosmos_rpc = "http://${COSMOS_RPC}"
cosmos_grpc = "http://${COSMOS_GRPC}"
cosmos_base_dir = "${COSMOS_DIR}"
namada_channel_id = "${namada_channel_id}"
cosmos_channel_id = "${cosmos_channel_id}"
EOF

echo "Initializing workload-${WORKLOAD_ID} state..."

output=$(/app/namada-chain-workload initialize \
    --config config.toml \
    --no-check | tee /dev/stderr)
if echo "$output" | grep -q "Done initialize"
then
    echo "Initialization succeeded!"
else
    echo "Initialization failed!"
    exit 1
fi

output=$(/app/namada-chain-workload fund-all \
    --config config.toml | tee /dev/stderr)
if echo "$output" | grep -q "Done fund-all"
then
    # Ready to start workload
    touch /container_ready/workload-${WORKLOAD_ID}
    echo "Ready to start the workload"
else
    echo "Fund failed!"
    exit 1
fi

for id in $(seq 0 $(({WORKLOAD_NUM} - 1))); do
    while [ ! -f "/container_ready/workload-${id}" ]; do
        echo "Waiting for workload-${id} initialization..."
        sleep 2
    done
    echo "workload-${id} is ready"
done

# Emit `setup_complete` to Antithesis
if [ "${WORKLOAD_ID}" -eq 0 ]
then
  /app/namada-chain-workload --config config.toml new-wallet-key-pair --setup-complete
fi

echo "All workloads are ready"
