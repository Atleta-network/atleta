#!/bin/bash

LOCKFILE=/tmp/backup_script.lock

if [ -e "$LOCKFILE" ]; then
    echo "Script is running now. Lockfile enabled."
    exit 1
fi

touch $LOCKFILE

source /root/.bashrc

script_dir="/sportchain/backup_script"
log_dir="$script_dir/logs"

mkdir -p $log_dir

log_file="$log_dir/backup_$(date +%F_%H-%M-%S).log"

exec > >(tee -a "$log_file") 2>&1

max_logs=23
log_count=$(ls -1q "$log_dir" | wc -l)

if [ "$log_count" -gt "$max_logs" ]; then
    oldest_log=$(ls -t "$log_dir" | tail -n 1)
    echo "Deleting oldest log: $oldest_log"
    rm "$log_dir/$oldest_log"
fi

current_date=$(date +%F_%H-%M-%S)

block=$(curl -s -H "Content-Type: application/json" -d '{"id":1, "jsonrpc":"2.0", "method": "chain_getBlock", "params":[]}' http://185.190.140.207:9944 | jq -r '.result.block.header.number')
block=$(echo $block | tr -d '"')

archive_name="backup_diego_${current_date}_${block}.tar.gz"

archive_folder_1="/sportchain/chain-data/diego/chains/testnet"

exclude_folder_1="/sportchain/chain-data/diego/chains/testnet/keystore"
exclude_folder_2="/sportchain/chain-data/diego/chains/testnet/network"

bucket_name="atleta-testnet-bucket-bucket-644829d8753ee27c"
bucket_path="gs://${bucket_name}"

num_backups=23
storage_capacity=2199023255552

echo "Start script: $current_date"

tar -czPf $archive_name --exclude=$exclude_folder_1 --exclude=$exclude_folder_2 $archive_folder_1

stat -c %s $archive_name

gcloud auth activate-service-account --key-file="$script_dir/gcp_key.json"

file_count=$(gsutil du ${bucket_path} | wc -l)
echo "Number of files in bucket: $file_count"

storage_count=$(gsutil du -s ${bucket_path} | awk '{print $1}')
echo "Busy bytes in bucket: $storage_count"

while [ $file_count -gt $num_backups ] || [ $storage_count -gt $storage_capacity ]
do
    oldest_file=$(gsutil ls -l ${bucket_path} | tail -n +2 | sort -k2 | head -n 1 | awk '{print $NF}')
    echo "Deleting oldest backup: $oldest_file"
    gsutil rm $oldest_file
    ((file_count--))
    file_count=$(gsutil du ${bucket_path} | wc -l)
    storage_count=$(gsutil du -s ${bucket_path} | awk '{print $1}')
done

gsutil -o GSUtil:parallel_composite_upload_threshold=100M cp $archive_name $bucket_path

rm -rf $archive_name

rm $LOCKFILE

current_date=$(date +%F_%H-%M-%S)
echo "End script: $current_date"
