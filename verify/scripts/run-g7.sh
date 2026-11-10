#!/usr/bin/env bash
# G7 — smart-HTTP remote client (loopback/recorded) + G1–G6 regression.
set -euo pipefail

cd /workspace

if ! command -v git >/dev/null; then
  echo "G7: stock git missing in image" >&2
  exit 1
fi
echo "G7: stock git $(git --version)"
echo "G7: rustc $(rustc --version)"
echo "G7: CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-2}"

export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"

echo "G7: gix-exit scan"
if cargo metadata --format-version 1 --no-deps 2>/dev/null | grep -E '"name":\s*"(gix|gitoxide)' >/dev/null; then
  echo "G7: forbidden gix/gitoxide package name in workspace members" >&2
  exit 1
fi
# Full graph (with deps) for gix edges
meta="$(cargo metadata --format-version 1)"
if echo "$meta" | grep -E '"name":\s*"gix|"name":\s*"gitoxide|"name":\s*"gix-' >/dev/null; then
  echo "G7: forbidden gix/gitoxide edge in cargo metadata" >&2
  exit 1
fi
echo "G7: gix-exit OK"

echo "G7: build arks binary"
cargo build -p arks

echo "G7: remote smart-HTTP suite (verify/g7)"
cargo test --manifest-path verify/g7/Cargo.toml "$@"

echo "G7: arks-remote + arks unit tests"
cargo test -p arks-remote "$@"
cargo test -p arks "$@"

echo "G7: G6 Wave L argv regression (verify/g6)"
cargo test --manifest-path verify/g6/Cargo.toml "$@"

echo "G7: G3 index-commit regression"
cargo test --manifest-path verify/g3/Cargo.toml "$@"

echo "G7: G4/G5/G2/G10/G1 regression"
cargo test --manifest-path verify/g4/Cargo.toml "$@"
cargo test --manifest-path verify/g5/Cargo.toml "$@"
cargo test -p arks-protocol "$@"
cargo test --manifest-path verify/g2/Cargo.toml "$@"
cargo test --manifest-path verify/g10/Cargo.toml "$@"
cargo test -p arks-object --test g1_oracle "$@"
cargo test --manifest-path verify/g1/Cargo.toml "$@"

echo "G7: PASS"
