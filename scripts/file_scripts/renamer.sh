#!/bin/bash

if [[ $# -ne 1 ]]; then
    echo "Usage: $0 [base-dir]"
    exit 1
fi

if [[ "$1" != "" ]]; then
    BASE_DIR="$1/watch-dir"
else
    BASE_DIR="watch-dir"
fi

[[ -e "$BASE_DIR" ]] || mkdir "$BASE_DIR"

SRC_DIR=${BASE_DIR}/from
DEST_DIR=${BASE_DIR}/to

# Remove the dirs if already present
rm -rf $SRC_DIR
rm -rf $DEST_DIR

# Start the watcher
OUT_FILE=$(mktemp -p /tmp "event_printXXXXXX")
cargo build --release --example event_print
TARGET_DIR=${CARGO_TARGET_DIR:-./target}
"${TARGET_DIR}"/release/examples/event_print $BASE_DIR | tee "$OUT_FILE" &

echo -e "\033[33;1mCreating $SRC_DIR\033[0m"
mkdir $SRC_DIR
sleep 2

FILE_CNT=10
for i in $(seq $FILE_CNT); do
    FILE_NAME=${SRC_DIR}/file${i}
    echo -e "\033[33;1mCreating $FILE_NAME\033[0m"
    echo "Huzzah $i" > "$FILE_NAME"
    sleep 1
done

sleep 2
echo -e "\033[33;1mMoving $SRC_DIR to $DEST_DIR\033[0m"
mv $SRC_DIR $DEST_DIR

sleep 3
kill %1

exit 0
