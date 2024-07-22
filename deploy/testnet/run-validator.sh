#!/bin/bash
# Runs a validator node:
#
# 1. Starts an archive node;
# 2. Adds session keys;

# expects files in the same directory:
# - config.env
# - chainspec.json

# config.env should be created and uploaded by CD worker, it should contain:

# BOOTNODE_ADDRESS=<node address in libp2p form>
# PRIVATE_KEY=<key in hex>
# DOCKER_IMAGE=<image name>

set -u

source ./config.env

container_name="honest_worker"
chainspec="./chainspec.json"
rpc_api_endpoint="http://127.0.0.1:9944"

check_chainspec() {
    if [ ! -f "$chainspec" ]; then
        printf "\033[31mError: Chainspec file not found.\033[0m\n"
        exit 1
    fi
}

maybe_cleanup() {

    if [ "$(docker ps -q -f name=$container_name)" ]; then
        echo "Stopping existing container..."
        docker stop $container_name
    fi

    if [ "$(docker ps -aq -f name=$container_name)" ]; then
        echo "Removing existing container..."
        docker rm $container_name
    fi
}

start_node() {
    echo "Starting the validator node..."
    docker pull "$DOCKER_IMAGE"
    docker run -d --name "$container_name" \
        -v "$chainspec":"/chainspec.json" \
        -v "$(pwd)/chain-data":"/chain-data" \
        -p 30333:30333 \
        -p 9944:9944 \
        --platform linux/amd64 \
        --restart always \
        "$DOCKER_IMAGE" \
        --chain "/chainspec.json" \
        --validator \
        --name "Atleta Validator" \
        --unsafe-force-node-key-generation \
        --bootnodes "$BOOTNODE_ADDRESS" \
        --base-path /chain-data \
        --rpc-port 9944 \
        --unsafe-rpc-external \
        --rpc-methods=safe \
        --prometheus-external \
        --rpc-cors all \
        --allow-private-ipv4 \
        --listen-addr /ip4/0.0.0.0/tcp/30333 \
        --state-pruning archive \
        --enable-log-reloading \
        --max-runtime-instances 32 \
        --rpc-max-connections 10000
}

wait_availability() {
    local retry_count=0
    local max_retries=30
    local retry_interval=7

    while [ $retry_count -lt $max_retries ]; do

        # Use curl to test the connection without making an actual request and Check the exit status of curl
        if curl --connect-timeout 5 "$rpc_api_endpoint" 2>/dev/null; then
            echo "Connected to $rpc_api_endpoint"
            break
        else
            echo "$rpc_api_endpoint is not available. Retrying in $retry_interval seconds..." 
            sleep "$retry_interval"
            ((retry_count++))
        fi
    done
    
    if [ "$retry_count" -eq "$max_retries" ]; then
        printf "\033[31mError: Couldn't connect to %s\033[0m\n" "$rpc_api_endpoint"
        kill $$
    fi
}

check_chainspec
maybe_cleanup
start_node
wait_availability

# the rest is done via js

# FIXME: it doesn't work via CD.
# I don't know why yet, but the script always fails via CD, while working when
# you run it manually on the server.

# npm i 
# npm run set_keys
# npm run validate
