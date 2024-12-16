#!/bin/bash
set -euxo pipefail
cd "$(git rev-parse --show-toplevel)"
source buildscripts/init.sh "$1"

cargo clippy --all-targets --target "${RUST_TARGET}" -- -D warnings
cargo fmt --all --check
