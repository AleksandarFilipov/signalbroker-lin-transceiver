#!/bin/bash

set -o errexit
set -o nounset
set -o pipefail
set -o xtrace

readonly TARGET_HOST=pi@192.168.0.129
readonly TARGET_PATH=/home/pi/lin-transceiver
readonly SOURCE_PATH=./target/armv7-unknown-linux-gnueabihf/debug/lin-transceiver

cargo build
rsync ${SOURCE_PATH} ${TARGET_HOST}:${TARGET_PATH}
ssh -t ${TARGET_HOST} ${TARGET_PATH}
