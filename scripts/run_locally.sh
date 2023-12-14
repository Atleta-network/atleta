#!/bin/sh
# runs dev localy
# there will be 2 nodes:
# 1. alice validator and bootnode
# 2. bob validator

cargo run -- --chain dev --force-authoring --rpc-cors=all --alice --tmp --node-key 0000000000000000000000000000000000000000000000000000000000000001 &
    cargo run -- --chain dev --force-authoring --rpc-cors=all --bob --tmp --port 30334 --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp
