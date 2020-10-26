#! /bin/bash

set -e
rustc --version && cargo --version
cargo build --release --target x86_64-unknown-linux-musl
strip target/x86_64-unknown-linux-musl/release/sirene
mkdir -p ./dist
cp target/x86_64-unknown-linux-musl/release/sirene ./dist/
