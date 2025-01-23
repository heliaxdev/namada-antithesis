FROM docker.io/rust:1.81-bookworm as builder
WORKDIR /app

ARG GIT_SHA

RUN apt-get -y update
RUN apt-get install -y make git-core libssl-dev pkg-config libclang-dev build-essential protobuf-compiler libudev-dev

RUN git clone https://github.com/anoma/namada-masp-indexer.git . && git checkout $GIT_SHA
RUN cargo build --release --package chain

FROM debian:bookworm-slim AS runtime
WORKDIR /app
COPY --from=builder /app/target/release/chain /app/chain

RUN apt-get update && apt-get install -y libpq5 wget postgresql curl

# preload the MASP params (since we don't haveinternet)
RUN mkdir -p /app/.masp-params

RUN wget -O /app/.masp-params/masp-convert.params https://github.com/anoma/masp-mpc/releases/download/namada-trusted-setup/masp-convert.params
RUN wget -O /app/.masp-params/masp-output.params https://github.com/anoma/masp-mpc/releases/download/namada-trusted-setup/masp-output.params
RUN wget -O /app/.masp-params/masp-spend.params https://github.com/anoma/masp-mpc/releases/download/namada-trusted-setup/masp-spend.params

WORKDIR /app

COPY --chmod=0755 entrypoint.sh /app/entrypoint.sh

CMD ["./entrypoint.sh"]