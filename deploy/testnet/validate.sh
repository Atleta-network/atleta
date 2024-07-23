#!/bin/bash

# 1. Runs node with unsafe methods;
# 2. Adds session keys and validate;
# 3. Stops the node.

# expects files in the same directory:
# - config.env
# - chainspec.json

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

run_unsafe() {
    echo "Starting the validator node with unsafe methods enabled..."
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
        --rpc-methods=unsafe \
        --prometheus-external \
        --rpc-cors all \
        --allow-private-ipv4 \
        --listen-addr /ip4/0.0.0.0/tcp/30333 \
        --state-pruning archive \
        --log warn \
        --enable-log-reloading \
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

add_session_keys_and_validate() {
    npm i 
    npm run set_keys
    npm run validate
}

stop_node() {
    echo "Done. Restart the node with ./run.sh"
    kill $$
}

check_chainspec
maybe_cleanup
run_unsafe
wait_availability
add_session_keys_and_validate
stop_node
