#!/bin/bash
set -euxo pipefail
cd "$(git rev-parse --show-toplevel)"

TAG=$1

BASENAME="hashgood-${TAG}"
FILENAME="${BASENAME}.tar.xz"

git archive "${TAG}" -o "${FILENAME}" --prefix="${BASENAME}/"

echo "GENERIC_ARTIFACT|${FILENAME}|Source Code"
echo "URL|Git Tag|https://code.octet-stream.net/hashgood/shortlog/refs/tags/${TAG}|${TAG}"
