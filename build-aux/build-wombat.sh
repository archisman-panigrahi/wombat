#!/bin/sh
set -eu

source_root=$1
build_root=$2
output_path=$3

mkdir -p "$(dirname "$output_path")"

cargo build \
  --release \
  --manifest-path "$source_root/Cargo.toml"

cp "$build_root/cargo-target/release/wombat" "$output_path"