#!/bin/bash
## #!/usr/bin/env bash
set -eux

uname -a
DESTDIR=/src/releases

mkdir -p $DESTDIR/bin
rm -rf $DESTDIR/bin/*

function build {
    echo Building for $1 to $3...

    rustup target add $1

    if [[ ! -f /build/$1/release/bliss-mixer ]]; then
        cargo build --locked --release --target $1
    fi

    $2 /build/$1/release/bliss-mixer && cp /build/$1/release/bliss-mixer $DESTDIR/$3
}

build arm-unknown-linux-gnueabihf arm-linux-gnueabihf-strip bin/bliss-mixer-armhf
build aarch64-unknown-linux-gnu aarch64-linux-gnu-strip bin/bliss-mixer-aarch64
cp scripts/bliss-mixer-arm $DESTDIR/bliss-mixer
