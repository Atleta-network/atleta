#!/bin/bash

# utility scripts which reverts unfinalized blocks

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

check_chainspec() {
    if [ ! -f "$chainspec" ]; then
        printf "\033[31mError: Chainspec file not found.\033[0m\n"
        exit 1
    fi
}

maybe_cleanup() {
    if [ "$(docker ps -q -f name=$container_name)" ]; then
        echo "Stopping existing container..."
        docker stop "$container_name"
    fi

    if [ "$(docker ps -aq -f name=$container_name)" ]; then
        echo "Removing existing container..."
        docker rm "$container_name"
    fi
}

revert() {
    echo "Reverting..."
    docker pull "$DOCKER_IMAGE"
    docker run -d --name "$container_name" \
        -v "$chainspec":"/chainspec.json" \
        -v "$(pwd)/chain-data":"/chain-data" \
        --platform linux/amd64 \
        "$DOCKER_IMAGE" \
        revert \
        --chain "/chainspec.json" \
        --base-path /chain-data \
        --state-pruning archive
}

check_chainspec
maybe_cleanup
revert
