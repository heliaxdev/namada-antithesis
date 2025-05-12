#!/bin/bash

set -e

##
# This is a startup script used initialize a 3 validator namada dev network.
# It produces 3 base directories that can be used by individual validators to start/join the ledger node
# IMPORTANT: the working directory should be in /namada as it will save the chain tarball and copy the wasm artifacts
##

# Ensure the directories exist
mkdir -p /network-config # output from `namadac utils init network` command + the $CHAIN_ID.tar.gz archive
mkdir -p /validator-0 # base dir for validators that can be used for `namadac utils join network` 
mkdir -p /validator-1
mkdir -p /validator-2
mkdir -p /fullnode

# ENVARS we expect from the docker-compose file
# We should be able to use either hostname or ip address
VALIDATOR0_ADDR="${VALIDATOR0_ADDR:-30.0.0.12:27657}"
VALIDATOR1_ADDR="${VALIDATOR1_ADDR:-30.0.0.13:27657}"
VALIDATOR2_ADDR="${VALIDATOR2_ADDR:-30.0.0.14:27657}"

# Some variables for the setup
base_dirs=('/validator-0' '/validator-1' '/validator-2')
validator_aliases=('billy' 'bob' 'ben')
validator_voting_powers=('2000000' '2000000' '15000000')
validator_address=($VALIDATOR0_ADDR $VALIDATOR1_ADDR $VALIDATOR2_ADDR)

# https://github.com/heliaxdev/namada-network-templates/tree/master/devnet/it-se
network_template_path='/network-templates'

# https://github.com/anoma/namada
namada_path='/namada'
network_config_path='/network-config'

len=${#validator_aliases[@]}

# Generate validator keys
for ((i = 0; i < len; i++)); do
    echo Generating validator key in "${base_dirs[i]}" for "${validator_aliases[i]}"
    namadaw --base-dir "${base_dirs[i]}" --pre-genesis gen --alias "${validator_aliases[i]}" --unsafe-dont-encrypt
done

# Preparing the validators
for ((i = 0; i < len; i++)); do
    UNSIGNED_TX_FILE_PATH="${base_dirs[i]}/unsigned-tx.txt"
    SIGNED_TX_FILE_PATH="${base_dirs[i]}/signed-tx.txt"
    echo Making pre-genesis transactions for "${validator_aliases[i]}"

    # Doing some string parsing on the first line of the output like
    # Sad sed
    ESTABLISHED_ADDRESS=$(namadac utils \
        init-genesis-established-account \
        --base-dir "${base_dirs[i]}" \
        --path "${UNSIGNED_TX_FILE_PATH}" \
        --aliases ${validator_aliases[i]} | sed -n '1p' | sed -e 's/\x1b\[[0-9;]*m//g' | sed 's/Derived established account address: //')

    echo created established account ${ESTABLISHED_ADDRESS} for ${validator_aliases[i]}

    echo running init-genesis-validator command for "${validator_aliases[i]}"

    namadac utils \
    init-genesis-validator \
        --base-dir "${base_dirs[i]}" \
        --address ${ESTABLISHED_ADDRESS} \
        --alias ${validator_aliases[i]} --net-address ${validator_address[i]} \
        --commission-rate 0.05 --max-commission-rate-change 0.01 \
        --self-bond-amount ${validator_voting_powers[i]} --email ${validator_aliases[i]} \
        --path "${UNSIGNED_TX_FILE_PATH}" --unsafe-dont-encrypt

    # Sign the transactions
    namadac --base-dir ${base_dirs[i]} utils sign-genesis-txs --path ${UNSIGNED_TX_FILE_PATH} --output ${SIGNED_TX_FILE_PATH} --alias ${validator_aliases[i]}

    #5. Take all your signed transactions and put them together into one big `transactions.toml` in the template directory
    echo "Combining signed transactions into ${network_template_path}/transactions.toml for ${validator_aliases[i]}"
    echo "" >> ${network_template_path}/transactions.toml
    cat $SIGNED_TX_FILE_PATH >> ${network_template_path}/transactions.toml

    #6. Edit the `balances.toml` file to give a balance to each newly created established account (depends on the validator index, but 5k should be enough)
    echo "Adding balance to ${network_template_path}/balances.toml for ${validator_aliases[i]}"
    echo "" >> ${network_template_path}/balances.toml
    echo ${ESTABLISHED_ADDRESS} = '"18000000"' >> ${network_template_path}/balances.toml

done

# TODO: update the actual file
sed -i 's/epochs_per_year = 10512000/epochs_per_year = 105120/g' ${network_template_path}/parameters.toml
sed -i 's/default_mint_limit = .*/default_mint_limit = "1000000000000000000"/g' ${network_template_path}/parameters.toml
sed -i 's/default_per_epoch_throughput_limit = .*/default_per_epoch_throughput_limit = "1000000000000000000"/g' ${network_template_path}/parameters.toml

# 7. Start the chain
CHAIN_PREFIX="devnet"
GENESIS_TIME=$(date -Iseconds)
WASM_CHECKSUMS_PATH="${namada_path}/wasm/checksums.json"
namadac --base-dir=${network_config_path} utils init-network --chain-prefix ${CHAIN_PREFIX} --genesis-time ${GENESIS_TIME} --templates-path ${network_template_path} --wasm-checksums-path ${WASM_CHECKSUMS_PATH} --consensus-timeout-commit 2s

# Get the CHAIN ID from the release archive
CHAIN_ID=$(find ${namada_path}/ -type f -name "devnet*" | sed 's/namada//' | tr -d '/')
CHAIN_ID=$(basename "$CHAIN_ID" .tar.gz)

# Provide the chain ID for the workload to read
touch /container_ready/$CHAIN_ID

# Copy the tar archive to the network_config_path (Assuming we only have tar.gz in the namada path generated)
cp ${namada_path}/*.tar.gz ${network_config_path}

# Extract the archive to be used below
tar xzvf $network_config_path/$CHAIN_ID.tar.gz

# 8. Initialize each validator
for ((i = 0; i < len; i++)); do
    NAMADA_NETWORK_CONFIGS_DIR=$network_config_path namadac --base-dir ${base_dirs[i]} utils join-network --chain-id $CHAIN_ID --genesis-validator ${validator_aliases[i]} --pre-genesis-path ${base_dirs[i]}/pre-genesis/${validator_aliases[i]} --add-persistent-peers

    # Copy all of the wasm artifacts from the chain into base directory for each validator chain directory
    rm -rf ${base_dirs[i]}/${CHAIN_ID}/wasm
    cp -r ${namada_path}/${CHAIN_ID}/wasm ${base_dirs[i]}/${CHAIN_ID}/
    
    # Let each validator know it's ready to start 
    touch /container_ready/validator-${i}
done

#9. Initialize full node
NAMADA_NETWORK_CONFIGS_DIR=$network_config_path namadac --base-dir /fullnode utils join-network --chain-id $CHAIN_ID

# Copy all of the wasm artifacts from the chain into base directory for each fullnode chain directory
rm -rf /fullnode/${CHAIN_ID}/wasm
cp -r ${namada_path}/${CHAIN_ID}/wasm /fullnode/${CHAIN_ID}/

# Let each fullnode know it's ready to start 
touch /container_ready/fullnode

# So that container don't exit
echo Finished genesis ceremony, going to sleep now...

sleep infinity
