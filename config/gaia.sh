#!/bin/sh

CHAIN_ID=gaia-0
BASE_DIR=/gaia-0

RPC_PORT=26657
GRPC_PORT=9090

STAKE="100000000000stake"
USER_COINS="${STAKE},1000000samoleans"

gaiad --home $BASE_DIR --chain-id $CHAIN_ID init $CHAIN_ID &> /dev/null
sleep 1

gaiad --home $BASE_DIR keys add validator --keyring-backend="test" --output json > $BASE_DIR/validator_seed.json 2>&1
sleep 1

gaiad --home $BASE_DIR keys add user --keyring-backend="test" --output json > $BASE_DIR/user_seed.json 2>&1
sleep 1

VALIDATOR=$(gaiad --home $BASE_DIR keys --keyring-backend="test" show validator -a)
gaiad --home $BASE_DIR genesis add-genesis-account $VALIDATOR $STAKE &> /dev/null
sleep 1

USER=$(gaiad --home $BASE_DIR keys --keyring-backend="test" show user -a)
gaiad --home $BASE_DIR genesis add-genesis-account $USER $USER_COINS &> /dev/null
sleep 1

gaiad --home $BASE_DIR genesis gentx validator --keyring-backend="test" --chain-id $CHAIN_ID $STAKE &> /dev/null
sleep 1

gaiad --home $BASE_DIR genesis collect-gentxs &> /dev/null
sleep 1

sed -i 's/timeout_commit = "5s"/timeout_commit = "1s"/g' $BASE_DIR/config/config.toml
sed -i 's/timeout_propose = "3s"/timeout_propose = "1s"/g' $BASE_DIR/config/config.toml
sed -i 's/minimum-gas-prices = ""/minimum-gas-prices = "0stake"/g' $BASE_DIR/config/app.toml

gaiad --home $BASE_DIR start --pruning=nothing --rpc.laddr="tcp://0.0.0.0:$RPC_PORT" --grpc.address="0.0.0.0:$GRPC_PORT" --log_level info
