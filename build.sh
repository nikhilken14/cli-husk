#!/usr/bin/env sh
set -eu
cd "$(dirname "$0")"
cargo build --release
printf 'Built: %s\n' "$(pwd)/target/release/husk"
