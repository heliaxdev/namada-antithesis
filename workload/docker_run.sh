#!/bin/bash

touch state-$WORKLOAD_ID.json
echo "" > state-$WORKLOAD_ID.json
touch /opt/antithesis/test/v1/namada/state-$WORKLOAD_ID.json
echo "" > /opt/antithesis/test/v1/namada/state-$WORKLOAD_ID.json

mkdir -p base/wallet-$WORKLOAD_ID
mkdir -p base/masp-$WORKLOAD_ID

mkdir -p /opt/antithesis/test/v1/namada/wallet-$WORKLOAD_ID
mkdir -p /opt/antithesis/test/v1/namada/masp-$WORKLOAD_ID

if [[ ! -v ANTITHESIS_OUTPUT_DIR ]]; then
    source /opt/antithesis/test/v1/namada/first_get_chainid.sh
    if [ $? -eq 0 ] 
    then 
        echo "<OK> init" 
    else 
        echo "<ERROR> init"
    fi
    
    while true
    do

        source /opt/antithesis/test/v1/namada/parallel_driver_create_wallet.sh
        source /opt/antithesis/test/v1/namada/parallel_driver_create_wallet.sh
        source /opt/antithesis/test/v1/namada/parallel_driver_create_wallet.sh
        source /opt/antithesis/test/v1/namada/parallel_driver_create_wallet.sh
        source /opt/antithesis/test/v1/namada/parallel_driver_create_wallet.sh
        if [ $? -eq 0 ] 
        then 
            echo "<OK> wallet" 
        else 
            echo "<ERROR> wallet"
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
            echo "<ERROR> faucet"
        fi

        source /opt/antithesis/test/v1/namada/parallel_driver_bond.sh
        if [ $? -eq 0 ] 
        then 
            echo "<OK> bond" 
        else 
            echo "<ERROR> bond"
        fi
        
        source /opt/antithesis/test/v1/namada/parallel_driver_transparent_transfer.sh
        if [ $? -eq 0 ] 
        then 
            echo "<OK> transparent transfer" 
        else 
            echo "<ERROR> transparent transfer"
        fi

        source /opt/antithesis/test/v1/namada/parallel_driver_init_account.sh
        if [ $? -eq 0 ] 
        then 
            echo "<OK> init account" 
        else 
            echo "<ERROR> init account"
        fi

        source /opt/antithesis/test/v1/namada/parallel_driver_redelegate.sh
        if [ $? -eq 0 ] 
        then 
            echo "<OK> redelegate" 
        else 
            echo "<ERROR> redelegate"
        fi

        source /opt/antithesis/test/v1/namada/parallel_driver_unbond.sh
        if [ $? -eq 0 ] 
        then 
            echo "<OK> unbond" 
        else 
            echo "<ERROR> unbond"
        fi

        source /opt/antithesis/test/v1/namada/parallel_driver_shielding.sh
        if [ $? -eq 0 ] 
        then 
            echo "<OK> shielding" 
        else 
            echo "<ERROR> shielding"
        fi

        source /opt/antithesis/test/v1/namada/parallel_driver_shielded.sh
        if [ $? -eq 0 ] 
        then 
            echo "<OK> shielded" 
        else 
            echo "<ERROR> shielded"
        fi

        source /opt/antithesis/test/v1/namada/parallel_driver_unshielding.sh
        if [ $? -eq 0 ] 
        then 
            echo "<OK> unshielding" 
        else 
            echo "<ERROR> unshielding"
        fi

        source /opt/antithesis/test/v1/namada/parallel_driver_claim_rewards.sh
        if [ $? -eq 0 ] 
        then 
            echo "<OK> claim rewards" 
        else 
            echo "<ERROR> claim rewards"
        fi

        source /opt/antithesis/test/v1/namada/parallel_driver_become_validator.sh
        if [ $? -eq 0 ] 
        then 
            echo "<OK> become validator" 
        else 
            echo "<ERROR> become validator"
        fi

        source /opt/antithesis/test/v1/namada/parallel_driver_change_metadata.sh
        if [ $? -eq 0 ] 
        then 
            echo "<OK> change metadata" 
        else 
            echo "<ERROR> change metadata"
        fi

        source /opt/antithesis/test/v1/namada/parallel_driver_bond_batch.sh
        if [ $? -eq 0 ] 
        then 
            echo "<OK> bond batch" 
        else 
            echo "<ERROR> bond batch"
        fi

        source /opt/antithesis/test/v1/namada/parallel_driver_random_batch.sh
        if [ $? -eq 0 ] 
        then 
            echo "<OK> random batch" 
        else 
            echo "<ERROR> random batch"
        fi
    done
else
    echo "ANTITHESIS_OUTPUT_DIR has the value: $ANTITHESIS_OUTPUT_DIR"

    sleep infinity
fi