#!/bin/bash

num_of_args=$#
priv_key="$1"
user_host="$2"

check_args() {
    if [ $num_of_args -ne 2 ]; then
        printf "\033[31m"
        echo "Error: wrong number of arguments"
        printf "\033[0m"
        usage
        exit 1
    fi
}

usage() {
    echo "Usage: ./setup-server.sh <priv_key> <user_host>"
    printf "\t<priv_key>  private ssh key to use\n"
    printf "\t<user_host> user@host to connect to\n"
}

copy_id() {
    ssh-copy-id -i "${priv_key}.pub" "$user_host"
}

run_remote_setup() {
    scp -i "$priv_key" ./_remote-setup.sh "$user_host:~/setup.sh"
    ssh -i "$priv_key" "$user_host" 'bash ~/setup.sh'
    ssh -i "$priv_key" "$user_host" 'rm ~/setup.sh'
}

check_args
copy_id
run_remote_setup
