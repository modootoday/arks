#!/usr/bin/env bash
# G21 — complete CLI flag symmetry (M16 Sy waves).
# SoT: interop-test-matrix G21; matrix: verify/matrices/matrix-g21.md; plan: 20261109043910
set -euo pipefail

cd /workspace

if ! command -v git >/dev/null; then
  echo "G21: stock git missing in image" >&2
  exit 1
fi
echo "G21: stock git $(git --version)"
echo "G21: rustc $(rustc --version)"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"

echo "G21: gix-exit / no-celir scan"
# Same honesty as G20/G15/G22: members name scan + default feature tree.
if cargo metadata --format-version 1 --no-deps 2>/dev/null | grep -E '"name":\s*"(gix|gitoxide)' >/dev/null; then
  echo "G21: forbidden gix/gitoxide package name in workspace members" >&2
  exit 1
fi
if cargo tree -p arks-store --edges normal 2>/dev/null | grep -Ei 'celir' >/dev/null; then
  echo "G21: FAIL — default arks-store tree has celir edge" >&2
  cargo tree -p arks-store --edges normal >&2 || true
  exit 1
fi
echo "G21: gix-exit / default tree CELIR-free OK"

export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/workspace/target}"

echo "G21: build arks (release)"
cargo build -p arks --release

mkdir -p verify/gates/g21/out
export ARKS_G21_RECEIPT_OUT="${ARKS_G21_RECEIPT_OUT:-/workspace/verify/gates/g21/out/receipt.json}"
export ARKS_BIN="${ARKS_BIN:-$CARGO_TARGET_DIR/release/arks}"
echo "G21: ARKS_BIN=$ARKS_BIN"

echo "G21: Sy suite (verify/gates/g21)"
cargo test --manifest-path verify/gates/g21/Cargo.toml --release "$@"

echo "G21: G20 regression (historical M15)"
cargo test --manifest-path verify/gates/g20/Cargo.toml --release "$@"

echo "G21: G14 regression (historical M9)"
cargo test --manifest-path verify/gates/g14/Cargo.toml "$@"

echo "G21: PASS"
