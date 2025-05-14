#!/bin/bash

MAX_RETRY_COUNT=15

touch state-$WORKLOAD_ID.json
echo "" > state-$WORKLOAD_ID.json
touch /opt/antithesis/test/v1/namada/state-$WORKLOAD_ID.json
echo "" > /opt/antithesis/test/v1/namada/state-$WORKLOAD_ID.json

mkdir -p base/wallet-$WORKLOAD_ID
mkdir -p base/masp-$WORKLOAD_ID

mkdir -p /opt/antithesis/test/v1/namada/wallet-$WORKLOAD_ID
mkdir -p /opt/antithesis/test/v1/namada/masp-$WORKLOAD_ID

source /opt/antithesis/test/v1/namada/init_script.sh

output=$(/opt/antithesis/test/v1/namada/parallel_driver_create_wallet.sh | tee /dev/stderr)
if echo "$output" | grep -q "Done new-wallet-keypair"
then
    echo "<OK> create wallet"
else
    echo "<ERROR> create wallet"
    exit 1
fi

output=$(/opt/antithesis/test/v1/namada/parallel_driver_faucet_transfer.sh | tee /dev/stderr)
if echo "$output" | grep -q "Done faucet-transfer"
then
    echo "<OK> faucet transfer"
else
    echo "<ERROR> faucet transfer"
    exit 1
fi

output=$(/opt/antithesis/test/v1/namada/parallel_driver_bond.sh | tee /dev/stderr)
if echo "$output" | grep -q "Done bond"
then
    echo "<OK> bond"
else
    echo "<ERROR> bond"
    exit 1
fi

output=$(/opt/antithesis/test/v1/namada/parallel_driver_init_account.sh | tee /dev/stderr)
if echo "$output" | grep -q "Done init-account"
then
    echo "<OK> init account"
else
    echo "<ERROR> init account"
    exit 1
fi

output=$(/opt/antithesis/test/v1/namada/parallel_driver_transparent_transfer.sh | tee /dev/stderr)
if echo "$output" | grep -q "Done transparent-transfer"
then
    echo "<OK> transparent transfer"
else
    echo "<ERROR> transparent transfer"
    exit 1
fi

output=$(/opt/antithesis/test/v1/namada/parallel_driver_update_account.sh | tee /dev/stderr)
if echo "$output" | grep -q "Done update-account"
then
    echo "<OK> update account" 
else
    echo "<ERROR> update account"
    exit 1
fi

retries=1
while [ $retries -le $MAX_RETRY_COUNT ]; do
    output=$(/opt/antithesis/test/v1/namada/parallel_driver_shielding.sh | tee /dev/stderr)
    if echo "$output" | grep -q "Done shielding"
    then
        echo "<OK> shielding" 
        break
    else
        retries=$((retries + 1))
        echo "<RETRY ${retries}/$MAX_RETRY_COUNT> shielding"
    fi
done
if [ $retries -gt $MAX_RETRY_COUNT ]
then
    echo "<ERROR> shielding"
    exit 1
fi

retries=1
while [ $retries -le $MAX_RETRY_COUNT ]; do
    output=$(/opt/antithesis/test/v1/namada/parallel_driver_shielded.sh | tee /dev/stderr)
    if echo "$output" | grep -q "Done shielded-transfer"
    then
        echo "<OK> shielded transfer" 
        break
    else
        retries=$((retries + 1))
        echo "<RETRY ${retries}/$MAX_RETRY_COUNT> shielded transfer"
    fi
done
if [ $retries -gt $MAX_RETRY_COUNT ]
then
    echo "<ERROR> shielded transfer"
    exit 1
fi

retries=1
while [ $retries -le $MAX_RETRY_COUNT ]; do
    output=$(/opt/antithesis/test/v1/namada/parallel_driver_unshielding.sh | tee /dev/stderr)
    if echo "$output" | grep -q "Done unshielding"
    then
        echo "<OK> unshielding" 
        break
    else
        retries=$((retries + 1))
        echo "<RETRY ${retries}/$MAX_RETRY_COUNT> unshielding"
    fi
done
if [ $retries -gt $MAX_RETRY_COUNT ]
then
    echo "<ERROR> unshielding"
    exit 1
fi

output=$(/opt/antithesis/test/v1/namada/parallel_driver_ibc_transfer_send.sh | tee /dev/stderr)
if echo "$output" | grep -q "Done ibc-transfer-send"
then
    echo "<OK> IBC transfer send"
else
    echo "<ERROR> IBC transfer send"
    exit 1
fi

output=$(/opt/antithesis/test/v1/namada/parallel_driver_ibc_transfer_recv.sh | tee /dev/stderr)
if echo "$output" | grep -q "Done ibc-transfer-recv"
then
    echo "<OK> IBC transfer recv"
else
    echo "<ERROR> IBC transfer recv"
    exit 1
fi

retries=1
while [ $retries -le $MAX_RETRY_COUNT ]; do
    output=$(/opt/antithesis/test/v1/namada/parallel_driver_ibc_shielding_transfer.sh | tee /dev/stderr)
    if echo "$output" | grep -q "Done ibc-shielding-transfer"
    then
        echo "<OK> IBC shielding transfer"
        break
    else
        retries=$((retries + 1))
        echo "<RETRY ${retries}/$MAX_RETRY_COUNT> IBC shielding transfer"
    fi
done
if [ $retries -gt $MAX_RETRY_COUNT ]
then
    echo "<ERROR> IBC shielding transfer"
    exit 1
fi

retries=1
while [ $retries -le $MAX_RETRY_COUNT ]; do
    output=$(/opt/antithesis/test/v1/namada/parallel_driver_ibc_unshielding_transfer.sh | tee /dev/stderr)
    if echo "$output" | grep -q "Done ibc-unshielding-transfer"
    then
        echo "<OK> IBC unshielding transfer"
        break
    else
        retries=$((retries + 1))
        echo "<RETRY ${retries}/$MAX_RETRY_COUNT> IBC unshielding transfer"
    fi
done
if [ $retries -gt $MAX_RETRY_COUNT ]
then
    echo "<ERROR> IBC unshielding transfer"
    exit 1
fi

output=$(/opt/antithesis/test/v1/namada/parallel_driver_become_validator.sh | tee /dev/stderr)
if echo "$output" | grep -q "Done become-validator"
then
    echo "<OK> become validator" 
else
    echo "<ERROR> become validator"
    exit 1
fi

output=$(/opt/antithesis/test/v1/namada/parallel_driver_change_metadata.sh | tee /dev/stderr)
if echo "$output" | grep -q "Done change-metadata"
then
    echo "<OK> change metadata" 
else
    echo "<ERROR> change metadata"
    exit 1
fi

output=$(/opt/antithesis/test/v1/namada/parallel_driver_change_consensus_keys.sh | tee /dev/stderr)
if echo "$output" | grep -q "Done change-consensus-key"
then
    echo "<OK> change consensus keys" 
else
    echo "<ERROR> change consensus keys"
    exit 1
fi

output=$(/opt/antithesis/test/v1/namada/parallel_driver_bond_batch.sh | tee /dev/stderr)
if echo "$output" | grep -q "Done batch-bond"
then
    echo "<OK> bond batch" 
else
    echo "<ERROR> bond batch"
    exit 1
fi

retries=1
while [ $retries -le $MAX_RETRY_COUNT ]; do
    output=$(/opt/antithesis/test/v1/namada/parallel_driver_random_batch.sh | tee /dev/stderr)
    if echo "$output" | grep -q "Done batch-random"
    then
        echo "<OK> random batch" 
        break
    else
        retries=$((retries + 1))
        echo "<RETRY ${retries}/$MAX_RETRY_COUNT> random batch"
    fi
done
if [ $retries -gt $MAX_RETRY_COUNT ]
then
    echo "<ERROR> random batch"
    exit 1
fi

echo "Test was completed successfully"
