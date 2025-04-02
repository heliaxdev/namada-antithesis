#!/bin/sh

GAIA_CHAIN_ID=gaia-0

NAMADA_CHAIN_ID=$(find /container_ready -type f -name "devnet*")
while [ -z "$NAMADA_CHAIN_ID" ]
do
    echo Waiting for the chain ID
    NAMADA_CHAIN_ID=$(find /container_ready -type f -name "devnet*")
    sleep 2
done

NAMADA_CHAIN_ID=$(basename $NAMADA_CHAIN_ID)

# Wait for the RPC server starts
json_rpc_ready=0
while [ "$json_rpc_ready" != 200 ]
do
    echo "Checking node rpc ${RPC_ADDRESS}/status ..."
    json_rpc_ready=$(wget --server-response --spider "${RPC_ADDRESS}/status" 2>&1 | awk '/^  HTTP/{print $2}')
    echo "Node rpc query result: $json_rpc_ready"
    sleep 2
done


HERMES_CONFIG_TEMPLATE="
[global]
log_level = 'debug'

[mode]

[mode.clients]
enabled = true
refresh = true
misbehaviour = true

[mode.connections]
enabled = false

[mode.channels]
enabled = false

[mode.packets]
enabled = true
clear_interval = 10
clear_on_start = false
tx_confirmation = true

[telemetry]
enabled = false
host = '127.0.0.1'
port = 3001

[[chains]]
id = '_CHAIN_ID_'
type = 'Namada'
rpc_addr = 'http://_RPC_'
grpc_addr = 'http://30.0.0.14:9090'
event_source = { mode = 'push', url = 'ws://_RPC_/websocket', batch_delay = '500ms' }
account_prefix = ''
key_name = 'faucet'
store_prefix = 'ibc'
gas_price = { price = 0.000001, denom = 'tnam1qy440ynh9fwrx8aewjvvmu38zxqgukgc259fzp6h' }

[[chains]]
id = 'gaia-0'
type = 'CosmosSdk'
rpc_addr = 'http://30.0.0.31:26657'
grpc_addr = 'http://30.0.0.31:9090'
event_source = { mode = 'push', url = 'ws://30.0.0.31:26657/websocket', batch_delay = '500ms' }
account_prefix = 'cosmos'
key_name = 'user'
store_prefix = 'ibc'
gas_price = { price = 1.0, denom = 'stake' }
gas_multiplier = 1.3
"

echo "${HERMES_CONFIG_TEMPLATE}" \
  | sed -e "s/_CHAIN_ID_/$NAMADA_CHAIN_ID/g" \
  | sed -e "s/_RPC_/$RPC_ADDRESS/g" \
  > config.toml

hermes --config config.toml keys add --chain $NAMADA_CHAIN_ID --key-file /$TARGET_VALIDATOR/$NAMADA_CHAIN_ID/wallet.toml --overwrite
hermes --config config.toml keys add --chain $GAIA_CHAIN_ID --key-file /gaia-0/user_seed.json --overwrite

result=$(hermes --config config.toml \
  create channel --a-chain $NAMADA_CHAIN_ID \
  --b-chain $GAIA_CHAIN_ID \
  --a-port transfer \
  --b-port transfer \
  --new-client-connection --yes)

# not used for now
namada_channel_id=$(echo $result | sed -n 's/.*a_side:.*channel_id: Some( ChannelId( "\([^"]*\)".*/\1/p')
gaia_channel_id=$(echo $result | sed -n 's/.*b_side:.*channel_id: Some( ChannelId( "\([^"]*\)".*/\1/p')

hermes --config config.toml start
