#!/usr/bin/bash


echo "entered entrypoint"
until pg_isready -h 30.0.0.21 -p 5432 | grep 'accepting connections'; do
  echo "Waiting for PostgreSQL to be ready..."
  sleep 3  # Wait for 3 seconds before checking again
done
echo "PostgreSQL is now accepting connections!"

# Wait for the JSON RPC to come up for some validator
json_rpc_ready=0
while [ $json_rpc_ready != 200 ]
do
    echo "Checking node rpc ${COMETBFT_URL}/status ..."
    json_rpc_ready=$(curl -s -o /dev/null -w "%{http_code}" "${COMETBFT_URL}/status")
    echo "Node rpc query result: $json_rpc_ready"
    sleep 2
done

./transaction
