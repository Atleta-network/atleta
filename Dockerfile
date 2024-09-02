# Stage 1: Build the application

FROM rust:latest AS builder

ARG BUILD_FEATURES

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
    ntp \
    libpq-dev

RUN rustup target add wasm32-unknown-unknown
RUN rustup component add rustfmt clippy rust-src

WORKDIR /app

# Copy the project files
COPY . .

# Build the application
RUN cargo build --features "$BUILD_FEATURES" --locked --release


# Stage 2: Create the final image

FROM ubuntu:latest

EXPOSE 9944

RUN apt update -y && apt install -y curl

# Set the working directory
WORKDIR /app

# Copy the built binary from the builder stage
COPY --from=builder /app/target/release/lib* /app/target/release/atleta-node /app/bin/

ENTRYPOINT ["/app/bin/atleta-node"]
