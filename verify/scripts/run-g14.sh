#!/usr/bin/env bash
# G14 — Wave Pc inventory / flag-parity / alias-ready + G13/G12/G7 regression + gix-exit.
# CLI-parity production (M9).
set -euo pipefail

cd /workspace

if ! command -v git >/dev/null; then
  echo "G14: stock git missing in image" >&2
  exit 1
fi
echo "G14: stock git $(git --version)"
echo "G14: rustc $(rustc --version)"
echo "G14: CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-2}"

export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"

echo "G14: gix-exit scan"
if cargo metadata --format-version 1 --no-deps 2>/dev/null | grep -E '"name":\s*"(gix|gitoxide)' >/dev/null; then
  echo "G14: forbidden gix/gitoxide package name in workspace members" >&2
  exit 1
fi
meta="$(cargo metadata --format-version 1)"
if echo "$meta" | grep -E '"name":\s*"gix|"name":\s*"gitoxide|"name":\s*"gix-' >/dev/null; then
  echo "G14: forbidden gix/gitoxide edge in cargo metadata" >&2
  exit 1
fi
echo "G14: gix-exit OK"

echo "G14: build arks binary"
cargo build -p arks

echo "G14: Wave Pc parity suite (verify/g14)"
cargo test --manifest-path verify/g14/Cargo.toml "$@"

echo "G14: arks-remote + arks unit tests"
cargo test -p arks-remote "$@"
cargo test -p arks "$@"

echo "G14: G13 Wave Pb regression (verify/g13)"
cargo test --manifest-path verify/g13/Cargo.toml "$@"

echo "G14: G12 Wave Pa regression (verify/g12)"
cargo test --manifest-path verify/g12/Cargo.toml "$@"

echo "G14: G7 Wave H regression (verify/g7)"
cargo test --manifest-path verify/g7/Cargo.toml "$@"

echo "G14: G6 Wave L regression (verify/g6)"
cargo test --manifest-path verify/g6/Cargo.toml "$@"

echo "G14: G3/G4/G5/G2/G10/G1 regression"
cargo test --manifest-path verify/g3/Cargo.toml "$@"
cargo test --manifest-path verify/g4/Cargo.toml "$@"
cargo test --manifest-path verify/g5/Cargo.toml "$@"
cargo test -p arks-protocol "$@"
cargo test --manifest-path verify/g2/Cargo.toml "$@"
cargo test --manifest-path verify/g10/Cargo.toml "$@"
cargo test -p arks-object --test g1_oracle "$@"
cargo test --manifest-path verify/g1/Cargo.toml "$@"

echo "G14: PASS"
