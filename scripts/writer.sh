#!/bin/bash

# Script to create a file, write to it a few times and then remove it.
# Also run the watcher program with "cargo run"

set -e

DIR_NAME=watch-dir
FILE="$DIR_NAME"/watch-file

# Remove the file if it exists
[[ -e "$FILE" ]] && rm "$FILE"

# Start the watcher
cargo build --release
./target/release/notify-test "$DIR_NAME" &

# Create the file
touch "$FILE"
sleep 10

TIMES=10
WAIT_SECS=1s

# Write to it $TIMES times, watiting for $WAIT_SECS in between
for i in $(seq $TIMES); do
    echo "Hola $i" | tee -a "$FILE"
    sleep $WAIT_SECS
done

sleep 5
rm $FILE

# Stop the cargo process
sleep 10
kill %1

exit 0
