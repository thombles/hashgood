#!/bin/bash
set -euxo pipefail
cd "$(git rev-parse --show-toplevel)"

APP=hashgood

PLATFORM=$1
TAG=$2
source buildscripts/init.sh "${PLATFORM}"

BASENAME="${APP}-${TAG}-${PLATFORM}"

case $PLATFORM in
windows-x86_64)
    FILENAME="${BASENAME}.zip"
    TARCMD="/c/Windows/System32/tar.exe -acf ${FILENAME} ${BASENAME}"
    ;;
mac-x86_64|mac-arm64)
    FILENAME="${BASENAME}.pkg"
    TARCMD="pkgbuild --identifier net.octet-stream.${APP} --install-location /usr/local/bin/ --root ./${BASENAME} ${FILENAME}"
    ;;
*)
    FILENAME="${BASENAME}.tar.xz"
    TARCMD="tar -Jcf ${FILENAME} ${BASENAME}"
    ;;
esac


cargo build --target "${RUST_TARGET}" --release

if [[ ${CODESIGNCMD:-"unset"} != "unset" ]]; then
    "${CODESIGNCMD}" "target/${RUST_TARGET}/release/${APP}"
fi

cd target
mkdir "${BASENAME}"
mv "${RUST_TARGET}/release/${APP}" "${BASENAME}"
${TARCMD}

if [[ ${NOTARISECMD:-"unset"} != "unset" ]]; then
    "${NOTARISECMD}" "target/${FILENAME}"
fi

echo "PLATFORM_ARTIFACT|target/${FILENAME}"
