#!/bin/bash

set -e

if [[ ! -v ANTITHESIS ]]; then
    while true
    do
        ./opt/antithesis/test/v1/namada/first_get_chainid.sh

        ./opt/antithesis/test/v1/namada/parallel_driver_create_wallet.sh
        ./opt/antithesis/test/v1/namada/parallel_driver_create_wallet.sh
        ./opt/antithesis/test/v1/namada/parallel_driver_create_wallet.sh
        ./opt/antithesis/test/v1/namada/parallel_driver_create_wallet.sh
        ./opt/antithesis/test/v1/namada/parallel_driver_create_wallet.sh

        ./opt/antithesis/test/v1/namada/parallel_driver_faucet_transfer.sh
        ./opt/antithesis/test/v1/namada/parallel_driver_faucet_transfer.sh
        ./opt/antithesis/test/v1/namada/parallel_driver_faucet_transfer.sh
        ./opt/antithesis/test/v1/namada/parallel_driver_faucet_transfer.sh
        ./opt/antithesis/test/v1/namada/parallel_driver_faucet_transfer.sh

        ./opt/antithesis/test/v1/namada/parallel_driver_bond.sh
        
        ./opt/antithesis/test/v1/namada/parallel_driver_transparent_transfer.sh

        ./opt/antithesis/test/v1/namada/parallel_driver_init_account.sh

        ./opt/antithesis/test/v1/namada/parallel_driver_bond_batch.sh

        ./opt/antithesis/test/v1/namada/parallel_driver_random_batch.sh
    done
else
    echo "ANTITHESIS has the value: $ANTITHESIS"

    sleep infinity
fi