#!/usr/bin/env bash
# G11 — lease-product (M10) + G14/G7 regression + gix-exit.
# Lease capability bar (does not redefine M9 CLI-parity production).
set -euo pipefail

cd /workspace

if ! command -v git >/dev/null; then
  echo "G11: stock git missing in image" >&2
  exit 1
fi
echo "G11: stock git $(git --version)"
echo "G11: rustc $(rustc --version)"
echo "G11: CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-2}"

export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"

echo "G11: gix-exit scan"
if cargo metadata --format-version 1 --no-deps 2>/dev/null | grep -E '"name":\s*"(gix|gitoxide)' >/dev/null; then
  echo "G11: forbidden gix/gitoxide package name in workspace members" >&2
  exit 1
fi
meta="$(cargo metadata --format-version 1)"
if echo "$meta" | grep -E '"name":\s*"gix|"name":\s*"gitoxide|"name":\s*"gix-' >/dev/null; then
  echo "G11: forbidden gix/gitoxide edge in cargo metadata" >&2
  exit 1
fi
if echo "$meta" | grep -Ei '"name":\s*"celir|"name":\s*"celir-' >/dev/null; then
  echo "G11: forbidden celir edge in cargo metadata" >&2
  exit 1
fi
echo "G11: gix-exit / no-celir OK"

echo "G11: build arks binary"
cargo build -p arks

echo "G11: arks-lease unit tests (L1/L2/dual-path/lock-additive)"
cargo test -p arks-lease "$@"

echo "G11: lease-product suite (verify/g11)"
cargo test --manifest-path verify/g11/Cargo.toml "$@"

echo "G11: G14 CLI-parity regression (verify/g14)"
cargo test --manifest-path verify/g14/Cargo.toml "$@"

echo "G11: G7 remote-client regression (verify/g7)"
cargo test --manifest-path verify/g7/Cargo.toml "$@"

echo "G11: PASS"
