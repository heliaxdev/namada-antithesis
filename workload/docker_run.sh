#!/bin/bash

set -e

touch state-$WORKLOAD_ID.json
echo "" > state-$WORKLOAD_ID.json
touch /opt/antithesis/test/v1/namada/state-$WORKLOAD_ID.json
echo "" > /opt/antithesis/test/v1/namada/state-$WORKLOAD_ID.json

if [[ ! -v ANTITHESIS_OUTPUT_DIR ]]; then
    while true
    do
        source /opt/antithesis/test/v1/namada/first_get_chainid.sh
        if [ $? -eq 0 ] 
        then 
            echo "<OK> init" 
        else 
            echo "<ERROR> init" >&2 
        fi

        source /opt/antithesis/test/v1/namada/parallel_driver_create_wallet.sh
        source /opt/antithesis/test/v1/namada/parallel_driver_create_wallet.sh
        source /opt/antithesis/test/v1/namada/parallel_driver_create_wallet.sh
        source /opt/antithesis/test/v1/namada/parallel_driver_create_wallet.sh
        source /opt/antithesis/test/v1/namada/parallel_driver_create_wallet.sh
        if [ $? -eq 0 ] 
        then 
            echo "<OK> wallet" 
        else 
            echo "<ERROR> wallet" >&2 
        fi

        source /opt/antithesis/test/v1/namada/parallel_driver_faucet_transfer.sh
        source /opt/antithesis/test/v1/namada/parallel_driver_faucet_transfer.sh
        source /opt/antithesis/test/v1/namada/parallel_driver_faucet_transfer.sh
        source /opt/antithesis/test/v1/namada/parallel_driver_faucet_transfer.sh
        source /opt/antithesis/test/v1/namada/parallel_driver_faucet_transfer.sh
        if [ $? -eq 0 ] 
        then 
            echo "<OK> faucet" 
        else 
            echo "<ERROR> faucet" >&2 
        fi

        source /opt/antithesis/test/v1/namada/parallel_driver_bond.sh
        if [ $? -eq 0 ] 
        then 
            echo "<OK> bond" 
        else 
            echo "<ERROR> bond" >&2 
        fi
        
        source /opt/antithesis/test/v1/namada/parallel_driver_transparent_transfer.sh
        if [ $? -eq 0 ] 
        then 
            echo "<OK> transparent transfer" 
        else 
            echo "<ERROR> transparent transfer" >&2 
        fi

        source /opt/antithesis/test/v1/namada/parallel_driver_init_account.sh
        if [ $? -eq 0 ] 
        then 
            echo "<OK> init accout" 
        else 
            echo "<ERROR> init account" >&2 
        fi

        source /opt/antithesis/test/v1/namada/parallel_driver_bond_batch.sh
        if [ $? -eq 0 ] 
        then 
            echo "<OK> bond batch" 
        else 
            echo "<ERROR> bond batch" >&2 
        fi

        source /opt/antithesis/test/v1/namada/parallel_driver_random_batch.sh
        if [ $? -eq 0 ] 
        then 
            echo "<OK> random batch" 
        else 
            echo "<ERROR> random batch" >&2 
        fi
    done
else
    echo "ANTITHESIS_OUTPUT_DIR has the value: $ANTITHESIS_OUTPUT_DIR"

    sleep infinity
fi