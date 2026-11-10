#!/usr/bin/env bash
set -euo pipefail
cd /workspace
unset RUSTUP_TOOLCHAIN || true
rustup component add rustfmt 2>/dev/null || true
cargo fmt --all -- --check
echo "fmt: OK"
