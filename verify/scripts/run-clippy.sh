#!/usr/bin/env bash
set -euo pipefail
cd /workspace
# Compose sets RUSTUP_TOOLCHAIN=1.99.1; that rustup channel may lack clippy.
# Prefer the image-default toolchain (already 1.99.1) which has components.
unset RUSTUP_TOOLCHAIN || true
echo "clippy: rustc $(rustc --version)"
rustup component add clippy 2>/dev/null || true
cargo clippy --workspace --all-targets -- -D warnings
echo "clippy: OK"
