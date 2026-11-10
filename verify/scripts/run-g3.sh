#!/usr/bin/env bash
# G3 — full index-commit round-trip vs stock git (requires arks bin).
set -euo pipefail

cd /workspace

if ! command -v git >/dev/null; then
  echo "G3: stock git missing in image" >&2
  exit 1
fi
echo "G3: stock git $(git --version)"
echo "G3: rustc $(rustc --version)"
echo "G3: CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-2}"

export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"

echo "G3: build arks binary"
cargo build -p arks

echo "G3: index-commit oracle (verify/g3)"
cargo test --manifest-path verify/g3/Cargo.toml "$@"
