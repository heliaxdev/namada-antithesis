FROM lukemathwalker/cargo-chef:latest-rust-1.81 AS chef
WORKDIR /app
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM lukemathwalker/cargo-chef:latest-rust-1.81 AS cacher
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
WORKDIR /app
COPY --from=chef /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

FROM lukemathwalker/cargo-chef:latest-rust-1.81 AS builder
WORKDIR /app
COPY . .
COPY --from=cacher /app/target target
RUN cargo build --release --locked

FROM docker.io/debian:bookworm-slim
RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install --no-install-recommends --assume-yes \
    ca-certificates \
    curl \
    build-essential \
    && apt-get clean && rm -rf /var/lib/apt/lists/*
WORKDIR /app

COPY --from=builder /app/target/release/namada-chain-workload /app/namada-chain-workload 
COPY --from=builder /app/docker_run.sh /app/run.sh
COPY --from=builder /app/scripts/ /opt/antithesis/test/v1/namada/

RUN chmod +x /app/run.sh && chmod -R +x /opt/antithesis/test/v1/namada/

RUN curl -o /app/masp-spend.params -L https://github.com/anoma/masp-mpc/releases/download/namada-trusted-setup/masp-spend.params\?raw\=true \
    && curl -o /app/masp-output.params -L https://github.com/anoma/masp-mpc/releases/download/namada-trusted-setup/masp-output.params?raw=true \
    && curl -o /app/masp-convert.params -L https://github.com/anoma/masp-mpc/releases/download/namada-trusted-setup/masp-convert.params?raw=true

ENV NAMADA_MASP_PARAMS_DIR=/app

ENTRYPOINT ["/app/run.sh"]
