#!/bin/bash

BASE_DIR=watch-dir
FILE_NAME=heck
FILE_PATH=${BASE_DIR}/${FILE_NAME}
DIR_PATH=${BASE_DIR}/${FILE_NAME}

[[ -e "$FILE_PATH" && -f "$FILE_PATH" ]] && rm -f $FILE_PATH
[[ -e "$DIR_PATH" && -d "$DIR_PATH" ]] && rm -rf $DIR_PATH

# Start the watcher
cargo build --release
./target/release/tuxdrive &

echo -e "\033[33;1mCreating $FILE_PATH file\033[0m"
touch $FILE_PATH
sleep 6

echo -e "\033[33;1mRemoving $FILE_PATH file\033[0m"
rm $FILE_PATH
echo -e "\033[33;1mCreating $DIR_PATH directory\033[0m"
mkdir $DIR_PATH
echo -e "\033[33;1mCreating children to $DIR_PATH directory\033[0m"
touch $DIR_PATH/file1
touch $DIR_PATH/file2


sleep 12
kill %1

exit 0
