#!/bin/sh

set -e

cd $(dirname $0)
~/.cargo/bin/cross build --target aarch64-unknown-linux-gnu --release
scp target/aarch64-unknown-linux-gnu/release/powerlog pi:/mnt/data/powerlog/
