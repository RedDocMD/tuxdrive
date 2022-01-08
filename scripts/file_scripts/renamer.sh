#!/bin/bash

BASE_DIR=watch-dir
SRC_DIR=${BASE_DIR}/from
DEST_DIR=${BASE_DIR}/to

# Remove the dirs if already present
rm -rf $SRC_DIR
rm -rf $DEST_DIR

# Start the watcher
cargo build --release
./target/release/tuxdrive &

echo -e "\033[33;1mCreating $SRC_DIR\033[0m"
mkdir $SRC_DIR
sleep 6

FILE_CNT=10
for i in $(seq $FILE_CNT); do
    FILE_NAME=${SRC_DIR}/file${i}
    echo -e "\033[33;1mCreating $FILE_NAME\033[0m"
    echo "Huzzah $i" > $FILE_NAME
    sleep 1
done

sleep 6
echo -e "\033[33;1mMoving $SRC_DIR to $DEST_DIR\033[0m"
mv $SRC_DIR $DEST_DIR

sleep 10
kill %1

exit 0
