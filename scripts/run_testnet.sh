#!/bin/bash
# run testnet locally

num_of_args=$#
base_path="$1"
envs="$2"

# network_pid is the global array of pids for all the nodes
network_pids=()
node=./target/debug/atleta-node

check_args() {
    if [ $num_of_args -ne 2 ]; then
        printf "\033[31mError: wrong number of arguments\033[0m\n"
        usage
        exit 1
    fi
}

usage() {
    echo "Usage: ./run_testnet.sh <base_path> <envs_file>"
    printf "\t<base_path> is the nodes storage directory\n"
    printf "\t<envs_file> contains the environment variables with session keys\n"
    printf "\n\033[31m"
    echo "The envs file should contain the variables:"
    printf "\t<DIEGO, PELE, FRANZ>_<BABE, GRAN, IMON>_<PRIVATE, PUBLIC>\n" 
    printf "\033[0m\n"
}

load_envs() {
    source "$envs"
}

print_info() {
    echo "About to run bootnode with RPC port 9944, and two additional nodes on" 
    echo "ports 9955 and 9966."
    sleep 3
}

start_network() {
    run_bootnode 9944
    run_node 9955 30344
    run_node 9966 30355

    # Wait for Ctrl-C
    trap 'exit' INT
}

stop_network() {
    for pid in "${network_pids[@]}"; do
        kill -KILL "$pid"
    done

    echo "Session keys added. Stopping network..."
    check_lock_file "bootnode"
    check_lock_file "node-9955"
    check_lock_file "node-9966"
}

run_bootnode() {
    local rpc_port="$1"

    "$node" \
        --chain testnet \
        --force-authoring \
        --rpc-cors=all \
        --validator \
        --rpc-port "$1" \
        --base-path "${base_path}/bootnode/" \
        --node-key 0000000000000000000000000000000000000000000000000000000000000001 &

    network_pids+=($!)
}

run_node() {
    local rpc_port="$1"
    local p2p_port="$2"

    "$node" \
        --chain testnet \
        --force-authoring \
        --rpc-cors=all \
        --validator \
        --rpc-port "$rpc_port" \
        --base-path "${base_path}/node-${rpc_port}/" \
        --port "$p2p_port" \
        --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp &

    network_pids+=($!)
}

check_lock_file() {
    local node_id=$1
    local file=$base_path/$node_id/chains/testnet/db/full/LOCK

    while lsof "$file" >/dev/null 2>&1; do
        sleep 1
    done
}

check_availability() {
    local rpc_api_endpoint=$1
    local retry_count=0
    local max_retries=30
    local retry_interval=10

    while [ $retry_count -lt $max_retries ]; do
      
        # Use curl to test the connection without making an actual request and Check the exit status of curl
        if curl --connect-timeout 10 "$rpc_api_endpoint"; then
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

add_session_keys() {
    local prefix="$1"
    local rpc_api_endpoint="$2"

    local private_babe="${prefix}_BABE_PRIVATE"
    local public_babe="${prefix}_BABE_PUBLIC"
    local private_gran="${prefix}_GRAN_PRIVATE"
    local public_gran="${prefix}_GRAN_PUBLIC"
    local private_imon="${prefix}_IMON_PRIVATE"
    local public_imon="${prefix}_IMON_PUBLIC"

    add_key "babe" "${!private_babe}" "${!public_babe}" "$rpc_api_endpoint"
    add_key "gran" "${!private_gran}" "${!public_gran}" "$rpc_api_endpoint"
    add_key "imon" "${!private_imon}" "${!public_imon}" "$rpc_api_endpoint"
}

add_key() {
    local key_type="$1"
    local private="$2"
    local public="$3"
    local rpc_api_endpoint="$4"
    
    local request="{\
        \"jsonrpc\":\"2.0\",\
        \"id\":1,\
        \"method\":\"author_insertKey\",\
        \"params\": [ \"$key_type\", \"$private\", \"$public\" ]\
    }"

    echo "Adding '${key_type}' key to ${rpc_api_endpoint}"
    curl -H "Content-Type: application/json" -d "$request" "$rpc_api_endpoint"
}

check_args
load_envs
print_info
cargo build

start_network

sleep 10
check_availability "http://localhost:9944"
check_availability "http://localhost:9955"
check_availability "http://localhost:9966"

add_session_keys "DIEGO" "http://localhost:9944"
add_session_keys "PELE" "http://localhost:9955"
add_session_keys "FRANZ" "http://localhost:9966"

# restart the network to make the keys effective
stop_network

echo "Restarting network..."
start_network

# Keep the script running until Ctrl-C
while :; do sleep 1; done
