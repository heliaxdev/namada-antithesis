FROM docker.io/rust:1.81-bookworm AS builder
WORKDIR /app

ARG GIT_SHA
RUN [ -z "$GIT_SHA" ] && echo "GIT_SHA is required" && exit 1 || true

RUN apt-get -y update && apt-get install -y make git-core libssl-dev pkg-config libclang-dev build-essential protobuf-compiler libudev-dev

ADD --keep-git-dir=true https://github.com/anoma/namada-masp-indexer.git#$GIT_SHA .
RUN cargo build --release --locked --package webserver

FROM debian:bookworm-slim AS runtime
WORKDIR /app

RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install --no-install-recommends -y \
    libpq5 \
    wget \
    postgresql \
    curl \
    ca-certificates \
    && apt-get clean && rm -rf /var/lib/apt/lists/*

RUN curl -o /app/masp-spend.params -L https://github.com/anoma/masp-mpc/releases/download/namada-trusted-setup/masp-spend.params\?raw\=true \
    && curl -o /app/masp-output.params -L https://github.com/anoma/masp-mpc/releases/download/namada-trusted-setup/masp-output.params?raw=true \
    && curl -o /app/masp-convert.params -L https://github.com/anoma/masp-mpc/releases/download/namada-trusted-setup/masp-convert.params?raw=true
ENV NAMADA_MASP_PARAMS_DIR=/app

COPY --from=builder /app/target/release/webserver /app/webserver
COPY --chmod=0755 entrypoint.sh /app/entrypoint.sh

CMD ["./entrypoint.sh"]