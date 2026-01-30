# Stage 1: build
FROM rust:1.92.0-trixie AS builder

ARG BUILD_FEATURES

WORKDIR /usr/src/app

RUN apt-get update -y && \
    apt-get install -y --no-install-recommends \
    build-essential \
    cmake \
    pkg-config \
    libssl-dev \
    git \
    clang \
    libclang-dev \
    protobuf-compiler \
    jq \
    chrony \
    libpq-dev && \
    rm -rf /var/lib/apt/lists/*

RUN rustup target add wasm32-unknown-unknown && \
    rustup component add rustfmt clippy rust-src

COPY . .

RUN cargo build --features "$BUILD_FEATURES" --locked --release

RUN rm -rf /usr/local/cargo/git /usr/local/cargo/registry

# Stage 2: runtime
FROM ubuntu:24.04 AS runner

RUN apt-get update -y && \
    apt-get install -y --no-install-recommends \
    tini \
    gosu \
    curl \
    ca-certificates \
    libc6 \
    libgcc-s1 \
    libstdc++6 \
    libssl3 \
    zlib1g && \
    rm -rf /var/lib/apt/lists/*

RUN groupadd --system --gid 1001 appuser && \
    useradd --system --uid 1001 --gid appuser --home /home/appuser --shell /usr/sbin/nologin appuser && \
    mkdir -p /home/appuser/.cache /chain-data && \
    chown -R 1001:1001 /home/appuser /chain-data

WORKDIR /app

COPY --from=builder /usr/src/app/target/release/lib* /app/bin/
COPY --from=builder /usr/src/app/target/release/atleta-node /app/bin/
COPY --from=builder /usr/src/app/target/release/polkadot-execute-worker /app/bin/
COPY --from=builder /usr/src/app/target/release/polkadot-prepare-worker /app/bin/

COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

EXPOSE 30333 9944 9615

ENTRYPOINT ["/usr/bin/tini", "--", "/entrypoint.sh"]
CMD ["/app/bin/atleta-node"]