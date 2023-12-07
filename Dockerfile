# Stage 1: Build the application

FROM rust:latest as builder

ARG PROFILE=release
WORKDIR /app

# Update system packages and install build dependencies
RUN apt update -y && \
    apt install -y \
    cmake \
    pkg-config \
    libssl-dev \
    git \
    gcc \
    build-essential \
    clang \
    libclang-dev \
    protobuf-compiler \
    jq \
    libpq-dev

# Install rust wasm. Needed for substrate wasm engine
RUN rustup target add wasm32-unknown-unknown

# Copy the project files
COPY . .

# Build the application
RUN cargo build --locked "--$PROFILE"

#Stage 2: Create the final image
FROM ubuntu:latest

# Set the working directory
WORKDIR /app

# Copy the built binary from the builder stage
COPY --from=builder /app/target/release/lib* /app/target/release/sportchain-node /app/target/release/


# 30333 for p2p traffic
# 9933 for RPC call
# 9944 for Websocket
# 9615 for Prometheus (metrics)
EXPOSE 30333 9933 9944 9615

ENTRYPOINT ["/app/target/release/sportchain-node"]
