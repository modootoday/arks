#!/usr/bin/env bash
# G6 — Wave L CLI argv suite vs stock git, then G3 + G1–G4 / G10 regression.
set -euo pipefail

cd /workspace

if ! command -v git >/dev/null; then
  echo "G6: stock git missing in image" >&2
  exit 1
fi
echo "G6: stock git $(git --version)"
echo "G6: rustc $(rustc --version)"
echo "G6: CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-2}"

export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"

echo "G6: build arks binary"
cargo build -p arks

echo "G6: Wave L argv suite (verify/g6)"
cargo test --manifest-path verify/g6/Cargo.toml "$@"

echo "G6: full G3 index-commit (verify/g3)"
cargo test --manifest-path verify/g3/Cargo.toml "$@"

echo "G6: G4 pack + G5 protocol + G2/G10/G1 regression"
cargo test --manifest-path verify/g4/Cargo.toml "$@"
cargo test --manifest-path verify/g5/Cargo.toml "$@"
cargo test -p arks-protocol "$@"
cargo test --manifest-path verify/g2/Cargo.toml "$@"
cargo test --manifest-path verify/g10/Cargo.toml "$@"
cargo test -p arks-object --test g1_oracle "$@"
cargo test --manifest-path verify/g1/Cargo.toml "$@"
