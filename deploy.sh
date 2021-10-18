#!/bin/bash

set -o errexit
set -o nounset
set -o pipefail
set -o xtrace

readonly PROJECT_NAME=signalbroker-lin-transceiver-rp
readonly TARGET_HOST=pi@$1
readonly TARGET_PATH=/home/pi/$PROJECT_NAME
readonly SOURCE_PATH=./target/armv7-unknown-linux-gnueabihf/release/$PROJECT_NAME

cargo build --release --target="armv7-unknown-linux-gnueabihf"
rsync ${SOURCE_PATH} ${TARGET_HOST}:${TARGET_PATH}
ssh -t ${TARGET_HOST} ${TARGET_PATH}