# Stage 1: Prepare the build environment
FROM lukemathwalker/cargo-chef:latest-rust-1.81 AS chef
WORKDIR /app
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Stage 2: Cache dependencies
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

# Stage 3: Build the application
FROM lukemathwalker/cargo-chef:latest-rust-1.81 AS builder
WORKDIR /app
COPY . .
COPY --from=cacher /app/target target
RUN cargo build --release --locked

# Stage 4: Create the final image
FROM docker.io/debian:bookworm-slim
RUN DEBIAN_FRONTEND=noninteractive apt-get update && apt-get install -y ca-certificates curl build-essential && apt-get clean
WORKDIR /app

# Copy the runtime files
COPY --from=builder /app/target/release/namada-chain-check /app/namada-chain-check 
COPY --chmod=0755 docker_run.sh /app/run.sh

# Download masp parameters
RUN curl -o /app/masp-spend.params -L https://github.com/anoma/masp-mpc/releases/download/namada-trusted-setup/masp-spend.params\?raw\=true \
    && curl -o /app/masp-output.params -L https://github.com/anoma/masp-mpc/releases/download/namada-trusted-setup/masp-output.params?raw=true \
    && curl -o /app/masp-convert.params -L https://github.com/anoma/masp-mpc/releases/download/namada-trusted-setup/masp-convert.params?raw=true

ENV NAMADA_MASP_PARAMS_DIR=/app

ENTRYPOINT ["/app/run.sh"]
