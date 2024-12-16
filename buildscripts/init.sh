#!/bin/bash
set -euxo pipefail
cd "$(git rev-parse --show-toplevel)"

PLATFORM=$1

case $PLATFORM in
mac-x86_64)
    RUST_TARGET=x86_64-apple-darwin
    ;;
mac-arm64)
    RUST_TARGET=aarch64-apple-darwin
    ;;
linux-x86_64)
    RUST_TARGET=x86_64-unknown-linux-gnu
    ;;
linux-armhf)
    RUST_TARGET=armv7-unknown-linux-gnueabihf
    ;;
linux-arm64)
    RUST_TARGET=aarch64-unknown-linux-gnu
    ;;
windows-x86_64)
    RUST_TARGET=x86_64-pc-windows-msvc
    ;;
*)
    echo "Unrecognised platform"
    exit 1
    ;;
esac

export RUST_TARGET
