#!/usr/bin/env bash
# G10 — store facade (filesystem Git backend) vs stock git + G1 regression.
set -euo pipefail

cd /workspace

if ! command -v git >/dev/null; then
  echo "G10: stock git missing in image" >&2
  exit 1
fi
echo "G10: stock git $(git --version)"
echo "G10: rustc $(rustc --version)"
echo "G10: CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-2}"

export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"

echo "G10: facade oracle (verify/g10)"
cargo test --manifest-path verify/g10/Cargo.toml "$@"

echo "G10: G1 regression (M2 facade must not break loose-object interop)"
cargo test -p arks-object --test g1_oracle "$@"
cargo test --manifest-path verify/g1/Cargo.toml "$@"
