#!/usr/bin/env bash
# G5 — protocol offline fixture replay in clean-room (no live network).
set -euo pipefail

cd /workspace

echo "G5: rustc $(rustc --version)"
echo "G5: CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-2}"

export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"

echo "G5: offline fixtures (verify/gates/g5)"
cargo test --manifest-path verify/gates/g5/Cargo.toml "$@"

echo "G5: arks-protocol unit + g5_offline"
cargo test -p arks-protocol "$@"
