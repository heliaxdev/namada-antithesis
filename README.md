# Namada Antithesis Setup

This is a fully containerized "devnet" that runs 3 validator nodes. It is primarily used for testing with the Antithesis simulation environment. 

## Use this

1. Make sure you build the corresponding container images 

* namada-genesis - Used to perform the genesis ceremony, initiate the network (creating the chain) and joining the validators
* namada - Mostly for running `namadan ledger run` command to start/join the ledger node

2. Run `docker-compose up` in the `config` directory

## How it works

1. The namada-genesis container performs the genesis ceremony and prepares the `base-dir` for all validators
2. It "tells" each validator when these preparations are ready by creating a file inside of a commonly volume mounted directory `container_ready`
3. Each validator container waits for its `base-dir` to be populated and start/joins the ledger node `namadan ledger run`

## todo

1. Verifying the stack is running cleanly and correctly
2. Incorporate a workload to run against this stack
4. (Optional) Optimize the container image size 

## Caveats/Observations/Feedbacks

1. The genesis ceremony needs to be run from the `namada` directory as its working directory (https://github.com/anoma/namada), this copies the right .wasm artifacts
2. The $CHAIN_ID.tar.gz archive is created in whatever working directory we are in...