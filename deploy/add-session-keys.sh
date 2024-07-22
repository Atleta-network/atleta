#!/bin/bash
# adds session keys to the node
# it's supposed that the node is already built and running

set -u

num_of_args=$#
envs="$1"
prefix="$2"
rpc_api_endpoint="$3"
container_name="$4"

check_args() {
    if [ $num_of_args -ne 4 ]; then
        printf "\033[31m"
        echo "Error: wrong number of arguments"
        printf "\033[0m"
        usage
        exit 1
    fi
}

usage() {
    echo "Usage: ./add-session-keys.sh <ENVS_FILE> <PREFIX> <RPC_API_ENDPOINT> <CONTAINER_NAME>"
    printf "\t<ENVS_FILE>        contains the environment variables with session keys\n"
    printf "\t<PREFIX>           the prefix of the environment variables in the envs file (for example, an account NAME_)\n"
    printf "\t<RPC_API_ENDPOINT> the URL to connect to the node via RPC\n"
    printf "\t<CONTAINER_NAME>   the name of the docker container in which the node is running\n"
    printf "\n\033[31m"
    echo "The envs file should contain the variables:"
    printf "\t<PREFIX><BABE, GRAN, IMON>_<PRIVATE, PUBLIC>"
    printf "\033[0m\n"
}

load_envs() {
    source "$envs"
    
    # check all variables are loaded
    local envs_postfix="BABE_PRIVATE BABE_PUBLIC GRAN_PRIVATE GRAN_PUBLIC IMON_PRIVATE IMON_PUBLIC"
    
    for postfix in $envs_postfix; do
        local variable_name="${prefix}${postfix}"

        if [[ -z "${!variable_name}" ]]; then
            printf "\033[31mError: %s is not set\033[0m\n" "$variable_name"
            exit 1
        fi
    done
}

check_availability() {
    local retry_count=0
    local max_retries=30
    local retry_interval=7

    while [ "$retry_count" -lt "$max_retries" ]; do

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

add_session_keys() {
    local babe_private="${prefix}BABE_PRIVATE"
    local babe_public="${prefix}BABE_PUBLIC"

    local gran_private="${prefix}GRAN_PRIVATE"
    local gran_public="${prefix}GRAN_PUBLIC"

    local imon_private="${prefix}IMON_PRIVATE"
    local imon_public="${prefix}IMON_PUBLIC"

    add_key "babe" "${!babe_private}" "${!babe_public}" "$rpc_api_endpoint"
    add_key "gran" "${!gran_private}" "${!gran_public}" "$rpc_api_endpoint"
    add_key "imon" "${!imon_private}" "${!imon_public}" "$rpc_api_endpoint"
}

add_key() {
    local key_type="$1"
    local private="$2"
    local public="$3"
    
    local request="{\
        \"jsonrpc\":\"2.0\",\
        \"id\":1,\
        \"method\":\"author_insertKey\",\
        \"params\": [ \"$key_type\", \"$private\", \"$public\" ]\
    }"

    echo "Adding '${key_type}' key to ${rpc_api_endpoint}"
    curl -X POST -H "Content-Type: application/json" -d "$request" "$rpc_api_endpoint"
}

restart_container() {
    docker restart "$container_name"
}

check_args
load_envs
check_availability
add_session_keys
restart_container

echo "Session keys are added"
