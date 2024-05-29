#!/bin/bash
# Runs a validator node:
#
# 1. Starts the archive node;
# 2. Adds session keys;

# expects files in the same directory:
# - config.env
# - chainspec.json

# config.env should be created and uploaded by CD worker, it should contain:

# BOOTNODE_ADDRESS=<node address in libp2p form>
# PRIVATE_KEY=<key in hex>
# DOCKER_IMAGE=<image name>

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

    if [ "$(docker ps -aq -f status=exited -f name=$container_name)" ]; then
        echo "Removing existing container..."
        docker rm $container_name
    fi
}

start_node() {
    echo "Starting the validator node..."
    docker run -d --name "$container_name" \
        -v "$chainspec":"/chainspec.json" \
        -p 30333:30333 \
        -p 9944:9944 \
        --platform linux/amd64 \
        "$DOCKER_IMAGE" \
        --chain "/chainspec.json" \
        --validator \
        --name "Atleta Validator" \
        --bootnodes "$BOOTNODE_ADDRESS" \
        --base-path ./chain-data \
        --rpc-port 9944 \
        --unsafe-rpc-external \
        --rpc-methods=Unsafe \
        --prometheus-external \
        --rpc-cors all \
        --allow-private-ipv4 \
        --listen-addr /ip4/0.0.0.0/tcp/30333 \
        --state-pruning archive
}

wait_availability() {
    local retry_count=0
    local max_retries=30
    local retry_interval=7

    while [ $retry_count -lt $max_retries ]; do
        # Use curl to test the connection without making an actual request
        curl --connect-timeout 5 "$rpc_api_endpoint" 2>/dev/null
        
        # Check the exit status of curl
        if [ $? -eq 0 ]; then
            echo "Connected to $rpc_api_endpoint"
            break
        else
            echo "$rpc_api_endpoint is not available. Retrying in $retry_interval seconds..." 
            sleep $retry_interval
            ((retry_count++))
        fi
    done
    
    if [ $retry_count -eq $max_retries ]; then
        printf "\033[31mError: Couldn't connect to %s\033[0m\n" "$rpc_api_endpoint"
        kill $$
    fi
}

check_chainspec
maybe_cleanup
start_node
wait_availability

# the rest is done via js
npm i && npm start
