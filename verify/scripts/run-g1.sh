#!/usr/bin/env bash
# G1 — arks loose-object interop vs in-image stock git (verify/g1 oracle tests).
set -euo pipefail

cd /workspace

if ! command -v git >/dev/null; then
  echo "G1: stock git missing in image" >&2
  exit 1
fi
echo "G1: stock git $(git --version)"
echo "G1: rustc $(rustc --version)"
echo "G1: CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-2}"

export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
# Workspace oracle tests (arks-object) + dedicated verify/g1 matrix runner.
cargo test -p arks-object --test g1_oracle "$@"
cargo test --manifest-path verify/g1/Cargo.toml "$@"
