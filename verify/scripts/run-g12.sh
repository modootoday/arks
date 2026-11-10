#!/usr/bin/env bash
# G12 — Wave Pa porcelain + G6/G7 regression + gix-exit.
set -euo pipefail

cd /workspace

if ! command -v git >/dev/null; then
  echo "G12: stock git missing in image" >&2
  exit 1
fi
echo "G12: stock git $(git --version)"
echo "G12: rustc $(rustc --version)"
echo "G12: CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-2}"

export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"

echo "G12: gix-exit scan"
if cargo metadata --format-version 1 --no-deps 2>/dev/null | grep -E '"name":\s*"(gix|gitoxide)' >/dev/null; then
  echo "G12: forbidden gix/gitoxide package name in workspace members" >&2
  exit 1
fi
meta="$(cargo metadata --format-version 1)"
if echo "$meta" | grep -E '"name":\s*"gix|"name":\s*"gitoxide|"name":\s*"gix-' >/dev/null; then
  echo "G12: forbidden gix/gitoxide edge in cargo metadata" >&2
  exit 1
fi
echo "G12: gix-exit OK"

echo "G12: build arks binary"
cargo build -p arks

echo "G12: Wave Pa porcelain suite (verify/gates/g12)"
cargo test --manifest-path verify/gates/g12/Cargo.toml "$@"

echo "G12: arks-remote + arks unit tests"
cargo test -p arks-remote "$@"
cargo test -p arks "$@"

echo "G12: G7 Wave H regression (verify/gates/g7)"
cargo test --manifest-path verify/gates/g7/Cargo.toml "$@"

echo "G12: G6 Wave L regression (verify/gates/g6)"
cargo test --manifest-path verify/gates/g6/Cargo.toml "$@"

echo "G12: G3/G4/G5/G2/G10/G1 regression"
cargo test --manifest-path verify/gates/g3/Cargo.toml "$@"
cargo test --manifest-path verify/gates/g4/Cargo.toml "$@"
cargo test --manifest-path verify/gates/g5/Cargo.toml "$@"
cargo test -p arks-protocol "$@"
cargo test --manifest-path verify/gates/g2/Cargo.toml "$@"
cargo test --manifest-path verify/gates/g10/Cargo.toml "$@"
cargo test -p arks-object --test g1_oracle "$@"
cargo test --manifest-path verify/gates/g1/Cargo.toml "$@"

echo "G12: PASS"
