FROM docker.io/rust:1.78 AS builder

COPY . /app

WORKDIR /app

RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install --no-install-recommends --assume-yes \
    libprotobuf-dev \
    build-essential \
    clang-tools-16 \
    git \
    libssl-dev \
    pkg-config \
    protobuf-compiler \
    libudev-dev \
    && apt-get clean

RUN cargo build --release

FROM docker.io/debian:bookworm-slim

RUN DEBIAN_FRONTEND=noninteractive apt-get update && apt-get install -y ca-certificates curl build-essential

WORKDIR /app

# copy the runtime files
COPY --from=builder /app/target/release/namada-chain-workload /app/namada-chain-workload 
COPY --chmod=0755 docker_run.sh /app/run.sh

# download masp parameters
RUN curl -o /app/masp-spend.params -L https://github.com/anoma/masp-mpc/releases/download/namada-trusted-setup/masp-spend.params\?raw\=true
RUN curl -o /app/masp-output.params -L https://github.com/anoma/masp-mpc/releases/download/namada-trusted-setup/masp-output.params?raw=true
RUN curl -o /app/masp-convert.params -L https://github.com/anoma/masp-mpc/releases/download/namada-trusted-setup/masp-convert.params?raw=true

ENV NAMADA_MASP_PARAMS_DIR=/app

ENTRYPOINT ["/app/run.sh"]
