#!/usr/bin/env bash

# Script for installing Atleta node automatically on Linux/MacOS.
# Example usage:
#     curl -sSL https://raw.githubusercontent.com/Atleta-network/atleta/bugfix/build-envs/scripts/run_validator_node.sh | bash

set -ue
set -o pipefail

# The path where the node files will be saved.
base_path=""
sudo_cmd=""
chain_spec_name="testnet"
keychain_exists=""
session_keys=""
release_version=""
binary_path="/opt/atleta"
binary_url=""

echo "This script will setup a Atleta Validator on your PC. Press Ctrl-C at any time to cancel."
echo "Detecting your architecture... "
operating_system=$(uname -s)
if [ "$operating_system" != "Linux" ] && [ "$operating_system" != "Darwin" ]; then
    echo "$operating_system is currently not supported by Atleta Node, only Linux or Darwin are supported. Exiting." >&2
    exit 1
else
    echo "OK ($operating_system)"
fi

echo "Checking privileges... "
if [ "$(id -u)" -eq 0 ]; then
    echo "OK (root)"
elif command -v sudo &> /dev/null; then
    echo "OK (sudo)"
    sudo_cmd=sudo
else
    echo "FAIL"
    echo "Not running as root and no sudo detected. Please run this as root or configure sudo. Exiting." >&2
    exit 1
fi

if [[ $operating_system == "Linux" ]]; then
    echo -n "Checking for systemd... "
    if [ -e /run/systemd/system ]; then
        echo "OK"
    else
        echo "FAIL"
        echo "No systemd detected. Exiting." >&2
        exit 1
    fi
fi

# Check for the 'jq' command
if ! command -v jq &>/dev/null; then
    echo "'jq' is not installed. Please install jq to continue."
    exit 1
fi

# Check for the 'curl' command
if ! command -v curl &>/dev/null; then
    echo "'curl' is not installed. Please install curl to continue."
    exit 1
fi

echo -n "Fetching release info... "
release_version=$(mktemp)
curl -Ls https://api.github.com/repos/Atleta-network/atleta/releases/latest -o $release_version
echo "OK ($(jq -r .name < $release_version))"

echo "All required dependencies are installed."

echo "Your choice is: $chain_spec_name network."

while [ -z "$base_path" ]; do
    read -r -p "Enter the path where you want to store at least a few gigabytes of data (or press Enter to use the standard $HOME/atleta/chain directory): " base_path

    if [ -z "$base_path" ]; then
        base_path="$HOME/atleta/chain"
        mkdir -p "$base_path"
        chmod -R a=rwx "$base_path"
        echo "Standard directory selected: $base_path"
    else
        base_path="$HOME/$base_path"
        mkdir -p "$base_path"
        chmod -R a=rwx "$base_path"
    fi

    if [ -w "$base_path" ]; then
        echo "The directory exists and you have write permissions."
    else
        echo "You do not have write permission to the specified directory. Please select another directory."
        base_path=""
    fi
done

if [ "$(ls -A "$base_path/chains" &>/dev/null)" ]; then
    keychain_exists=1
else
    keychain_exists=0
fi

echo "Everything's ready. Tasks:"
echo "  [X] Download atleta-node -> $binary_path/bin/atleta-node"
echo "  [X] Download $chain_spec_name network chain spec -> $binary_path/etc/chain_spec.$chain_spec_name.json"
if [ $keychain_exists -eq 0 ]; then
    echo "  [X] Generate new session keys and store them in node"
else
    echo "  [ ] Data dir already exists, so session keys won't be regenerated."
fi
echo "Press Enter to continue or Ctrl-C to cancel."
read -r < /dev/tty

echo "Create directories..."
$sudo_cmd mkdir -p "$binary_path/bin" "$binary_path/etc"
echo "Stop old node if it's running..."

#Check status of process
if [[ $operating_system == "Linux" ]]; then
    $sudo_cmd systemctl stop atleta-validator &>/dev/null || true
else
    $sudo_cmd launchctl unload /Library/LaunchDaemons/com.atleta.node.plist || true
fi

echo "Download binary..."

if [[ $operating_system == "Linux" ]]; then
    binary_url="linux_build"
else
    binary_url="macos_build"
fi

$sudo_cmd curl -sSL "https://github.com/Atleta-network/atleta/releases/download/$(jq -r .name < release_version)/$binary_url" -o "$binary_path/bin/atleta-node"
$sudo_cmd chmod +x "$binary_path/bin/atleta-node"
echo "Download chain spec..."
$sudo_cmd curl -sSL "https://raw.githubusercontent.com/Atleta-network/atleta/testnet/chainspecs/chain-spec.$chain_spec_name.json" -o "$binary_path/etc/chain_spec.$chain_spec_name.json"

echo "Generate atleta-validator.service..."

if [[ $operating_system == "Linux" ]]; then
    $sudo_cmd tee /etc/systemd/system/atleta-validator.service >/dev/null << EOF
    [Unit]
    Description=Atleta Validator Node Service
    After=network.target

    [Service]
    Type=simple
    ExecStart=$binary_path/bin/atleta-node --base-path $base_path --chain $binary_path/etc/chain_spec.$chain_spec_name.json --validator
    Restart=on-failure
    RestartSec=5m

    [Install]
    WantedBy=multi-user.target
EOF
$sudo_cmd systemctl daemon-reload
$sudo_cmd systemctl enable atleta-validator
$sudo_cmd systemctl start atleta-validator
else
    cat << EOF | $sudo_cmd tee /Library/LaunchDaemons/com.atleta.node.plist >/dev/null
    <?xml version="1.0" encoding="UTF-8"?>
    <!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
    <plist version="1.0">
    <dict>
        <key>Label</key>
        <string>com.atleta.node</string>
        <key>Program</key>
        <string>$binary_path/bin/atleta-node</string>
        <key>ProgramArguments</key>
        <array>
            <string>$binary_path/bin/atleta-node</string>
            <string>--chain</string>
            <string>$binary_path/etc/chain_spec.$chain_spec_name.json</string>
            <string>--validator</string>
            <string>--base-path</string>
            <string>$base_path</string>
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
$sudo_cmd launchctl load /Library/LaunchDaemons/com.atleta.node.plist
$sudo_cmd launchctl start com.atleta.node
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
        session_keys=$(curl -H "Content-Type: application/json" -d '{"id":1, "jsonrpc":"2.0", "method": "author_rotateKeys"}' http://127.0.0.1:9944/ 2>/dev/null | jq -r .result)
        (( i++ ))
        set -e
    done
    echo
fi

echo "Done!"
echo

if [[ $operating_system == "Linux" ]]; then
    cat << EOF
        Your node is now running. Useful commands:
            Check status: $sudo_cmd systemctl status atleta-validator
            Stop: $sudo_cmd systemctl stop atleta-validator
            Start: $sudo_cmd systemctl start atleta-validator
            Logs: $sudo_cmd journalctl -u atleta-validator
        Node data is stored in $base_path.
EOF
else
    cat << EOF
    Your node is now running. Useful commands:
        Check status: $sudo_cmd launchctl list | grep com.atleta.node
        Stop: $sudo_cmd launchctl unload /Library/LaunchDaemons/com.atleta.node.plist
        Start: $sudo_cmd launchctl load /Library/LaunchDaemons/com.atleta.node.plist
        Logs: cat /var/log/atleta-validator.log
    Node data is stored in $base_path.
EOF
fi

if [ $keychain_exists -eq 0 ]; then
    echo Session keys for your node: "$session_keys"
fi
