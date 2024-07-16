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
if [ -z "$1" ] || [ -z "$2" ] || [ -z "$3" ] || [ -z "$4" ]; then
  echo "Error. Usage: $0 <node_URL> <path_to_gcloud> <path_to_gcp_key_file> <bucket_name>"
  exit 1
fi

node_URL=$1
gcloud_path=$2
gcp_key_file=$3
bucket_name=$4

gcloud_bin="$gcloud_path/gcloud"
gsutil_bin="$gcloud_path/gsutil"

response=$(curl -s -H "Content-Type: application/json" -d '{"id":1, "jsonrpc":"2.0", "method": "system_health", "params":[]}' "${node_URL}")

# Checking the success of the response from node
if [ -n "$response" ]; then
  echo "The response from node has been received. Node status:"
  echo "$response" | jq
else
  echo "Error. No response was received from node"
  exit 1
fi

# Checking the path to gcloud
if ! command -v "${gcloud_bin}" &> /dev/null; then
  echo "Error. gcloud not found in: $gcloud_path"
  exit 1
fi

# Checking the gcp_key_file by the specified argument in CMD
if [ ! -f "$gcp_key_file" ]; then
  echo "Error. GCP key file $gcp_key_file not found."
  exit 1
fi

# Verification of authorization in GCP
if ! $gcloud_bin auth activate-service-account --key-file="$gcp_key_file"; then
  echo "Error. GCP service account could not be activated."
  exit 1
fi
script_dir="/sportchain/backup_script"
log_dir="$script_dir/logs"

mkdir -p $log_dir

log_file="$log_dir/backup_$(date +%F_%H-%M-%S).log"

exec > >(tee -a "$log_file") 2>&1 # Redirect stdout to log_file

max_logs=5
log_count=$(find "${log_dir}" -maxdepth 1 -type f | wc -l)

while [ $(find "${log_dir}" -maxdepth 1 -type f | wc -l) -gt "${max_logs}" ]; do
    oldest_log=$(find "${log_dir}" -maxdepth 1 -type f -printf '%T@ %P\n' | sort -n | head -n 1 | cut -d ' ' -f 2-)
    echo "Deleting oldest log: $oldest_log"
    rm -v "${log_dir}/${oldest_log}"
done

current_date=$(date +%F_%H-%M-%S)

block=$(curl -s -H "Content-Type: application/json" -d '{"id":1, "jsonrpc":"2.0", "method": "chain_getBlock", "params":[]}' "${node_URL}" | jq -r '.result.block.header.number')

# Checking the return of the last block number from node
if [ -z "${block}" ]; then
  echo "Error. Failed to get the block number from node."
  exit 1
fi

block=$(echo "${block}" | tr -d '"')
block=$(printf "%d" "${block}") #Convert from hex to decimal

archive_name="backup_diego_${current_date}_${block}.tar.gz"

archive_folder_1="/sportchain/chain-data/diego/chains/testnet"

exclude_folder_1="/sportchain/chain-data/diego/chains/testnet/keystore"
exclude_folder_2="/sportchain/chain-data/diego/chains/testnet/network"

bucket_path="gs://${bucket_name}"

max_backups_allowed=5 # S3-bucket num of backups quota
storage_capacity=2199023255552 # S3-bucket storage quota, in bytes (2TB)

echo "Start script: $current_date"

tar -czPf "$archive_name" --exclude=$exclude_folder_1 --exclude=$exclude_folder_2 $archive_folder_1

stat -c %s "$archive_name"

file_count=$($gsutil_bin du "${bucket_path}" | wc -l)
echo "Number of files in bucket: $file_count"

storage_count=$($gsutil_bin du -s "${bucket_path}" | awk '{print $1}')
echo "Busy bytes in bucket: $storage_count"

while [ "${file_count}" -ge "${max_backups_allowed}" ] || [ "${storage_count}" -gt "${storage_capacity}" ]
do
    oldest_file=$("${gsutil_bin}" ls -l "${bucket_path}" | tail -n +2 | sort -k2 | head -n 1 | awk '{print $NF}')
    echo "Deleting oldest backup: $oldest_file"
    $gsutil_bin rm "${oldest_file}"
    ((file_count--))
    storage_count=$($gsutil_bin du -s "${bucket_path}" | awk '{print $1}')
done

$gsutil_bin -o GSUtil:parallel_composite_upload_threshold=100M cp "${archive_name}" "${bucket_path}"

rm -rf "${archive_name}"

current_date=$(date +%F_%H-%M-%S) # Override current_date
echo "End script: $current_date"