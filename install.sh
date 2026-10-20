#!/usr/bin/env bash
# Install Fang. Requires cargo (Rust) and cc (gcc or clang).
set -e
command -v cc >/dev/null || { echo "fang: no C compiler found. Install gcc or clang."; exit 1; }
command -v cargo >/dev/null || { echo "fang: cargo not found. Install Rust: https://rustup.rs"; exit 1; }
cargo install --path "$(dirname "$0")/crates/fang-cli"
fang version
