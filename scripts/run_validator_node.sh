#!/bin/bash

sudo_cmd=""
files_path=""
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
operating_system=$(uname -s)
if [ "$operating_system" != "Linux" -a "$operating_system" != "Darwin" ]; then
	echo "$operating_system is currently not supported by Atleta Node, only Linux or Darwin are supported. Exiting." >&2
	exit 1
else
	echo "OK ($operating_system)"
fi

if [[ $arch == "Linux" ]]; then
  echo -n "Checking for systemd... "
  if [ -e /run/systemd/system ]; then
  	echo "OK"
  else
  	echo "FAIL"
  	echo "No systemd detected. Exiting." >&2
  	exit 1
  fi
fi

if ! command -v jq &>/dev/null || \
! command -v curl &>/dev/null
then
	echo "You need to install some dependencies before continuing:"
	echo "  jq curl"
	exit 1
fi

while [ "$chain_spec" == "" ]; do
    echo "Available networks: "
    echo "1) Devnet"
    echo "2) Testnet"
    echo "3) Mainnet"
    read -p "Select network for your running node: " choice
    case $choice in
        1)
            chain_spec="devnet"
            ;;
        2)
            chain_spec="testnet"
            ;;
        3)
            chain_spec="mainnet"
            ;;
        *)
            echo "Invalid choice. Please select a right number."
            continue
            ;;
    esac
done

echo "Your choice is: $chain_spec network."

while [ -z "$files_path" ]; do
    read -p "Enter the path where you want to store at least a few gigabytes of data (or press Enter to use the standard $HOME/atleta/chain directory): " files_path

    if [ -z "$files_path" ]; then
        files_path="$HOME/atleta/chain"
        $sudo_cmd mkdir -p "$files_path"
        $sudo_cmd chmod -R 777 "$files_path"
        echo "Standard directory selected: $files_path"
    else
        files_path="$HOME/$files_path"
        $sudo_cmd mkdir -p "$files_path"
        $sudo_cmd chmod -R 777 "$files_path"
    fi

    if [ -w "$files_path" ]; then
        echo "The directory exists and you have write permissions."
    else
        echo "You do not have write permission to the specified directory. Please select another directory."
        files_path=""
    fi
done

if [ "$(ls -A $files_path/chains &>/dev/null)" ]; then
	keychain_exists=1
else
	keychain_exists=0
fi

echo "Everything's ready. Tasks:"
echo "  [X] Download atleta-node -> ~/.config/atleta/atleta-node"
echo "  [X] Download $chain_spec network chain spec -> ~/.config/atleta/chain_spec.$chain_spec.json"
if [ $keychain_exists -eq 0 ]; then
	echo "  [X] Generate new session keys and store them in node"
else
	echo "  [ ] Data dir already exists, so session keys won't be regenerated."
fi
echo "Press Enter to continue or Ctrl-C to cancel."
read < /dev/tty

echo "Create directories..."
$sudo_cmd mkdir -p ~/.config/atleta/
echo "Stop old node if it's running..."

#Check status of process
if [[ $arch == "Linux" ]]; then
    $sudo_cmd systemctl stop atleta-validator &>/dev/null || true
else
    $sudo_cmd launchctl unload /Library/LaunchDaemons/com.example.atleta-validator.plist || true
fi

echo "Download binary..."
$sudo_cmd curl -sSL https://github.com/Atleta-network/atleta/releases/download/v1.0.0/atleta-node -o ~/.config/atleta/atleta-node
$sudo_cmd chmod +x /usr/local/bin/atleta-node
echo "Download chain spec..."
$sudo_cmd curl -sSL https://github.com/Atleta-network/atleta/releases/download/v1.0.0/chain_spec.$chain_spec.json -o ~/.config/atleta/chain_spec.$chain_spec.json

echo "Generate atleta-validator.service..."

if [[ $arch == "Linux" ]]; then
    $sudo_cmd tee /etc/systemd/system/atleta-validator.service >/dev/null << EOF
    [Unit]
    Description=Atleta Validator Node Service
    After=network.target

    [Service]
    Type=simple
    ExecStart=~/.config/atleta/atleta-node --base-path $files_path --chain ~/.config/atleta/chain_spec.$chain_spec.json --validator
    Restart=on-failure
    RestartSec=5m

    [Install]
    WantedBy=multi-user.target
EOF
    $sudo_cmd systemctl daemon-reload
    $sudo_cmd systemctl enable atleta-validator
    $sudo_cmd systemctl start atleta-validator
else
    cat << EOF | $sudo_cmd tee /Library/LaunchDaemons/com.example.atleta-validator.plist >/dev/null
    <?xml version="1.0" encoding="UTF-8"?>
    <!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
    <plist version="1.0">
    <dict>
        <key>Label</key>
        <string>com.example.atleta-validator</string>
        <key>Program</key>
        <string>~/.config/atleta/atleta-node</string>
        <key>ProgramArguments</key>
        <array>
            <string>~/.config/atleta/atleta-node</string>
            <string>--chain</string>
            <string>~/.config/atleta/chain_spec.$chain_spec.json</string>
            <string>--validator</string>
            <string>--base-path</string>
            <string>$files_path</string>
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
		session_keys=$(curl -H "Content-Type: application/json" -d '{"id":1, "jsonrpc":"2.0", "method": "author_rotateKeys"}' http://127.0.0.1:9944/ 2>/dev/null | jq -r .result)
		(( i++ ))
		set -e
	done
	echo
fi

echo "Done!"
echo

if [[ $arch == "Linux" ]]; then
    cat << EOF
        Your node is now running. Useful commands:
        	Check status: $sudo_cmd systemctl status atleta-validator
        	Stop: $sudo_cmd systemctl stop atleta-validator
        	Start: $sudo_cmd systemctl start atleta-validator
        	Logs: $sudo_cmd journalctl -u atleta-validator
        Node data is stored in /opt/atleta/data.
EOF
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