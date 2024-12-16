#!/bin/bash
set -euxo pipefail
cd "$(git rev-parse --show-toplevel)"
source buildscripts/init.sh "$1"

cargo test --target "${RUST_TARGET}"
