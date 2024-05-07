#!/bin/bash

set -euo pipefail

sudo_cmd=""
chain_spec=""
keychain_exists=""
session_keys=""

echo "This script will setup a Atleta Validator on your PC. Press Ctrl-C at any time to cancel."
echo "Checking privileges... "
if [ $(id -u) -eq 0 ]; then
	echo "OK (root)"
elif command -v sudo &> /dev/null; then
	echo "OK (sudo)"
	sudo_cmd=sudo
else
	echo "FAIL"
	echo "Not running as root and no sudo detected. Please run this as root or configure sudo. Exiting." >&2
	exit 1
fi

echo "Detecting your architecture... "
arch=$(uname -m)
if [ "$arch" != "x86_64" -a "$arch" != "arm64" ]; then
	echo "$arch is currently not supported by Atleta Node, only x86_64 or arm64 are supported. Exiting." >&2
	exit 1
else
	echo "OK ($arch)"
fi

chain_spec_exists=0
while [ "$chain_spec" == "" ]; do
    echo "Available networks: "
    i=0
    echo "1) Devnet"
    echo "2) Testnet"
    echo "Select network for your running node: "
    read choice < /dev/tty
    case $choice in
        1)
            chain_spec="dev"
            ;;
        2)
            chain_spec="testnet"
            ;;
        *)
            echo "Invalid choice. Please select a right number."
            continue
            ;;
    esac
done

echo "Your choose is: $chain_spec network."

if [ -n "$(ls -A /opt/atleta/data/chains &>/dev/null)" ]; then
	keychain_exists=1
else
	keychain_exists=0
fi

echo "Everything's ready. Tasks:"
echo "  [X] Download atleta-node -> /usr/local/bin/atleta-node"
echo "  [X] Download $chain_spec network chain spec -> /opt/atleta/chain_spec.json"
if [ $keychain_exists -eq 0 ]; then
	echo "  [X] Generate new session keys and store them in node"
else
	echo "  [ ] Data dir already exists, so session keys won't be regenerated."
fi
echo "Press Enter to continue or Ctrl-C to cancel."
read < /dev/tty

echo "Create directories..."
$sudo_cmd mkdir -p /opt/atleta/ /usr/local/bin/
echo "Stop old node if it's running..."

#Check status of process
if [[ $arch == "x86_64" ]]; then
    $sudo_cmd systemctl stop atleta-validator &>/dev/null || true
else
    $sudo_cmd launchctl unload /Library/LaunchDaemons/com.example.atleta-validator.plist || true
fi

echo "Download binary..."
$sudo_cmd curl -sSL https://github.com/Atleta-network/atleta/releases/download/v1.0.0/atleta-node -o /usr/local/bin/atleta-node
$sudo_cmd chmod +x /usr/local/bin/atleta-node
echo "Download chain spec..."
$sudo_cmd curl -sSL https://github.com/Atleta-network/atleta/releases/download/v1.0.0/$chain_spec-chain-spec.json -o /opt/atleta/chain_spec.json

echo "Generate atleta-validator.service..."

if [[ $arch == "x86_64" ]]; then
    $sudo_cmd systemctl stop atleta-validator &>/dev/null || true
else
    cat << EOF | $sudo_cmd tee /Library/LaunchDaemons/com.example.atleta-validator.plist >/dev/null
    <?xml version="1.0" encoding="UTF-8"?>
    <!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
    <plist version="1.0">
    <dict>
        <key>Label</key>
        <string>com.example.atleta-validator</string>
        <key>Program</key>
        <string>/usr/local/bin/atleta-node</string>
        <key>ProgramArguments</key>
        <array>
            <string>/usr/local/bin/atleta-node</string>
            <string>--chain</string>
            <string>/opt/atleta/chain_spec.json</string>
            <string>--validator</string>
        </array>
        <key>StandardErrorPath</key>
        <string>/var/log/atleta-validator.log</string>
        <key>RunAtLoad</key>
        <true/>
        <key>KeepAlive</key>
        <dict>
            <key>SuccessfulExit</key>
            <false/>
        </dict>
    </dict>
    </plist>
EOF
    $sudo_cmd launchctl load /Library/LaunchDaemons/com.example.atleta-validator.plist
    $sudo_cmd launchctl start com.example.atleta-validator
fi

echo "Atleta validator node was started successfully."

if [ $keychain_exists -eq 0 ]; then
	echo "Waiting for node to start to generate session keys (up to 5 minutes)..."
	i=0
	while [ -z "$session_keys" ]; do
		echo "Waiting..."
		if [ $i -gt 60 ]; then
			echo
			echo "Node didn't start after 5 minutes." >&2
			exit 1
		fi
		sleep 5
		set +e
		session_keys=$(wscat -c ws://127.0.0.1:9944 -x '{"id":1, "jsonrpc":"2.0", "method": "author_rotateKeys", "params":[]}' 2>/dev/null | jq -r .result)
		(( i++ ))
		set -e
	done
	echo
fi

echo "Done!"
echo

if [[ $arch == "x86_64" ]]; then
    $sudo_cmd systemctl stop atleta-validator &>/dev/null || true
else
    cat << EOF
    Your node is now running. Useful commands:
    	Check status: $sudo_cmd launchctl list | grep com.example.atleta-validator
    	Stop: $sudo_cmd launchctl unload /Library/LaunchDaemons/com.example.atleta-validator.plist
    	Start: $sudo_cmd launchctl load /Library/LaunchDaemons/com.example.atleta-validator.plist
    	Logs: cat /var/log/atleta-validator.log
    Node data is stored in /opt/atleta/data.
EOF
fi

if [ $keychain_exists -eq 0 ]; then
	echo Session keys for your node: $session_keys
fi