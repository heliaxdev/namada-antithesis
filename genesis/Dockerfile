# Following the instruction from https://docs.namada.net/introduction/install/source
# This is used as the genesis container to create the chain and join the validators to the network
FROM docker.io/rust:1.76-slim-bullseye as builder

# Pinning a commit
ARG GIT_SHA
ARG NETWORK_TEMPLATE_TAG
ARG PROFILE="release"

# problem with libclang-12-dev, also added curl
RUN apt-get -y update
RUN apt-get install -y make git-core libssl-dev pkg-config libclang-13-dev build-essential protobuf-compiler libudev-dev curl wget python3 python3-pip wget
RUN pip3 install toml

RUN git clone https://github.com/anoma/namada.git /namada && cd /namada && git checkout $GIT_SHA

# Important!
WORKDIR /namada

# Be aware the patch could potentially be version dependent
# getting rid of the sudo
COPY ./scripts_get_cometbft.patch .
RUN git apply scripts_get_cometbft.patch

RUN make install

RUN mkdir -p /container_ready

# Base dir for each validators 
RUN mkdir -p /validator-0
RUN mkdir -p /validator-1
RUN mkdir -p /validator-2

RUN cargo install wasm-opt --locked

RUN make build-wasm-scripts

# Preparing the network templates for starting the chain
RUN git clone https://github.com/heliaxdev/namada-network-templates.git /namada-network-templates
RUN mv /namada-network-templates/devnet/$NETWORK_TEMPLATE_TAG /network-templates

COPY ./entrypoint.sh /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]