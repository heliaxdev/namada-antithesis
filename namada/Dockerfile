# Deprecated in favor of the instrumented version
# Following the instruction from https://docs.namada.net/introduction/install/source
# This is so we can do the localnet setup
FROM docker.io/rust:1.76-slim-bullseye as builder

# Pinning a release tag
ARG GIT_SHA
ARG PROFILE="release"

# problem with libclang-12-dev, also added curl
RUN apt-get -y update
RUN apt-get install -y make git-core libssl-dev pkg-config libclang-13-dev build-essential protobuf-compiler libudev-dev curl wget

RUN git clone https://github.com/anoma/namada.git /namada && cd /namada && git checkout $GIT_SHA

WORKDIR /namada

# Be aware the patch could potentially be version dependent
# getting rid of the sudo
COPY ./scripts_get_cometbft.patch .
RUN git apply scripts_get_cometbft.patch

RUN make install

# preload the MASP params (since we don't have internet)
RUN mkdir -p /root/.masp-params

RUN wget -O /root/.masp-params/masp-convert.params https://github.com/anoma/masp-mpc/releases/download/namada-trusted-setup/masp-convert.params
RUN wget -O /root/.masp-params/masp-output.params https://github.com/anoma/masp-mpc/releases/download/namada-trusted-setup/masp-output.params
RUN wget -O /root/.masp-params/masp-spend.params https://github.com/anoma/masp-mpc/releases/download/namada-trusted-setup/masp-spend.params

# Try to slim down the runner
FROM docker.io/rust:1.76-slim-bullseye

RUN apt-get -y update
RUN apt-get install -y curl

COPY --from=builder /root/.masp-params/* /root/.masp-params/
COPY --from=builder /namada/target/release /namada/target/release
COPY --from=builder /usr/local/bin/cometbft /usr/local/bin/cometbft