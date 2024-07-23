#!/bin/bash

set -u

keys_file=$1

if [ $# -ne 1 ]; then
    printf "\033[31m"
    echo "Error: wrong number of arguments"
    printf "\033[0m"
    exit 1
fi

docker compose up -d

sleep 30

./add-session-keys.sh "$keys_file" DIEGO_ "http://127.0.0.1:9955" "sportchain-diego"&
./add-session-keys.sh "$keys_file" PELE_ "http://127.0.0.1:9966" "sportchain-pele"&
./add-session-keys.sh "$keys_file" FRANZ_ "http://127.0.0.1:9977" "sportchain-franz"

echo "Done"
