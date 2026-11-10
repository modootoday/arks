#!/usr/bin/env bash
# G33 - client credential-helper / askpass / TTY / OS wallets (FW-CRED-ASKPASS)
# Serialize cargo to avoid OOM when sibling agents contend for Docker RAM.
set -euo pipefail
cd /workspace
if ! command -v git >/dev/null; then
  echo "G33: stock git missing" >&2
  exit 1
fi
echo "G33: stock git $(git --version)"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/cargo-target-g33}"
export RUST_TEST_THREADS="${RUST_TEST_THREADS:-1}"
echo "G33: CARGO_TARGET_DIR=$CARGO_TARGET_DIR CARGO_BUILD_JOBS=$CARGO_BUILD_JOBS RUST_TEST_THREADS=$RUST_TEST_THREADS"
echo "G33: build arks"
cargo build -p arks -j1
echo "G33: arks-remote credential + wallet unit"
cargo test -p arks-remote --lib -j1 -- credential:: wallet:: --test-threads=1
echo "G33: credential / askpass suite (serialized)"
cargo test --manifest-path verify/gates/g33/Cargo.toml -j1 -- --test-threads=1 "$@"
echo "G33: PASS"
