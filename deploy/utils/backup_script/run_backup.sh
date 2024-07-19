#!/bin/bash

# This script may be added to crontab. Example usage:
# 0 * * * * /sportchain/backup_script/run_backup.sh <node_URL> <path_to_gcloud> <path_to_gcp_key_file> <S3-bucket name> >/dev/null 2>&1

set -e

LOCKFILE=/tmp/backup_script.lock

# The script will not start executing until the execution of the previous script is finished
if [ -e "$LOCKFILE" ]; then
    echo "Script is running now. Lockfile enabled."
    exit 1
fi

touch $LOCKFILE

terminate() {
    echo "Script terminated. Cleaning up."
    rm -f $LOCKFILE
    exit 1
}

# Set up trap to call terminate function on exit, SIGINT, and SIGTERM
trap terminate EXIT SIGINT SIGTERM

# Check whether the CMD arguments were passed
if [ "$#" -ne 3 ]; then
  echo "Error. Usage: $0 <node_URL> <path_to_gcp_key_file> <bucket_name>"
  exit 1
fi

node_URL=$1
gcp_key_file=$2
bucket_name=$3

response=$(curl -s -H "Content-Type: application/json" -d '{"id":1, "jsonrpc":"2.0", "method": "system_health", "params":[]}' "$node_URL")

# Checking the success of the response from node
if [ -n "$response" ]; then
  echo "The response from node has been received. Node status:"
  echo "$response" | jq
else
  echo "Error. No response was received from node"
  exit 1
fi

# Checking the path to gcloud
if ! command -v gcloud &> /dev/null; then
  echo "Error. gcloud not found"
  echo "deb [signed-by=/usr/share/keyrings/cloud.google.gpg] https://packages.cloud.google.com/apt cloud-sdk main" | tee -a /etc/apt/sources.list.d/google-cloud-sdk.list 
  curl https://packages.cloud.google.com/apt/doc/apt-key.gpg | gpg --dearmor --batch --yes -o /usr/share/keyrings/cloud.google.gpg 
  apt-get update -y && apt-get install google-cloud-sdk -y
else
  echo "gcloud -OK-"
fi

# Checking the gcp_key_file by the specified argument in CMD
if [ ! -f "$gcp_key_file" ]; then
  echo "Error. GCP key file $gcp_key_file not found."
  exit 1
fi

# Verification of authorization in GCP
if ! gcloud auth activate-service-account --key-file="$gcp_key_file"; then
  echo "Error. GCP service account could not be activated."
  exit 1
fi

atleta_dir="/home/${USER}/atleta"
script_dir="${atleta_dir}/backup_script"
log_dir="${script_dir}/logs"

mkdir -p "$log_dir"

current_date=$(date +%F_%H-%M-%S)

log_file="${log_dir}/backup_${current_date}.log"

exec > >(tee -a "$log_file") 2>&1 # Redirect stdout to log_file

max_logs=23

while [ "$(find "$log_dir" -maxdepth 1 -type f | wc -l)" -gt "$max_logs" ]; do
    oldest_log=$(find "$log_dir" -maxdepth 1 -type f -printf '%T@ %P\n' | sort -n | head -n 1 | cut -d ' ' -f 2-)
    echo "Deleting oldest log: $oldest_log"
    rm -v "$log_dir/$oldest_log"
done

block=$(curl -s -H "Content-Type: application/json" -d '{"id":1, "jsonrpc":"2.0", "method": "chain_getBlock", "params":[]}' "$node_URL" | jq -r '.result.block.header.number')

# Checking the return of the last block number from node
if [ -z "$block" ]; then
  echo "Error. Failed to get the block number from node."
  exit 1
fi

block=$(echo "$block" | tr -d '"')
block=$(printf "%d" "$block") # Convert from hex to decimal

archive_name="backup_diego_${current_date}_${block}.tar.gz"

source_rsync="${atleta_dir}/./chain-data/chains/testnet"
target_rsync="${script_dir}/backup_rsync"

archive_folder_1="${target_rsync}/chain-data/chains/testnet"

exclude_folder_1="${target_rsync}/chain-data/chains/testnet/keystore"
exclude_folder_2="${target_rsync}/chain-data/chains/testnet/network"

bucket_path="gs://$bucket_name"

max_backups_allowed=23 # S3-bucket num of backups quota
storage_capacity=2199023255552 # S3-bucket storage quota, in bytes (2TB)

echo "Start script: $current_date"

docker pause honest_worker # Pause container with blockchain worker

rsync -a -R "$source_rsync" "$target_rsync"

docker unpause honest_worker # Resume container with blockchain worker

tar -czPf "$archive_name" --exclude="$exclude_folder_1" --exclude="$exclude_folder_2" "$archive_folder_1" # Create archive with backup

stat -c %s "$archive_name"

file_count=$(gsutil du "$bucket_path" | wc -l)
echo "Number of files in bucket: $file_count"

storage_count=$(gsutil du -s "$bucket_path" | awk '{print $1}')
echo "Busy bytes in bucket: $storage_count"

while [ "$file_count" -ge "$max_backups_allowed" ] || [ "$storage_count" -gt "$storage_capacity" ]
do
    oldest_file=$(gsutil ls -l "$bucket_path" | tail -n +2 | sort -k2 | head -n 1 | awk '{print $NF}')
    echo "Deleting oldest backup: $oldest_file"
    gsutil rm "$oldest_file"
    ((file_count--))
    storage_count=$(gsutil du -s "$bucket_path" | awk '{print $1}')
done

gsutil -o GSUtil:parallel_composite_upload_threshold=100M cp "$archive_name" "$bucket_path"

rm -rf "$archive_name"
rm -rf "$target_rsync"
current_date=$(date +%F_%H-%M-%S) # Override current_date
echo "End script: $current_date"
