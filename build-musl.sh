#! /bin/bash

set -e
rustc --version && cargo --version
chown -R $(id -u):$(id -g) ./target /root/.cargo/registry /root/.cargo/git
cargo build --release --target x86_64-unknown-linux-musl
mkdir -p ./dist
cp target/x86_64-unknown-linux-musl/release/sirene ./dist/
strip ./dist/sirene
