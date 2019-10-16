#! /bin/bash

rustc --version && cargo --version
cargo build --release --target x86_64-unknown-linux-musl
strip target/x86_64-unknown-linux-musl/release/sirene
cp target/x86_64-unknown-linux-musl/release/sirene .
