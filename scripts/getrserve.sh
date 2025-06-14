#!/bin/sh

TEMP_FOLDER="/tmp/rserve"
BIN_FOLDER="$HOME/.local/bin"

git clone https://github.com/gustavosvalentim/rserve.git $TEMP_FOLDER

cd $TEMP_FOLDER

cargo build --release

mv ./target/release/rshttp $BIN_FOLDER

rm -rf $TEMP_FOLDER