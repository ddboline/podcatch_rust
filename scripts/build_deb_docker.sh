#!/bin/bash

VERSION="$1"
RELEASE="$2"

. ~/.cargo/env

ln -s /usr/bin/python3 /usr/bin/python

cargo build --release

printf "CLI Podcatcher written in Rust\n" > description-pak
echo checkinstall --pkgversion ${VERSION} --pkgrelease ${RELEASE} -y
checkinstall --pkgversion ${VERSION} --pkgrelease ${RELEASE} -y
