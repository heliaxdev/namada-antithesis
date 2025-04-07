#!/bin/bash

set -e

/app/namada-chain-workload --config config.toml ibc-transfer-send
