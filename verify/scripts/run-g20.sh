#!/usr/bin/env bash
# G20 — full CLI symmetric parity (M15 Fs waves).
# SoT: interop-test-matrix G20; matrix: verify/matrices/matrix-g20.md; plan: 20261109035335
set -euo pipefail

cd /workspace

if ! command -v git >/dev/null; then
  echo "G20: stock git missing in image" >&2
  exit 1
fi
echo "G20: stock git $(git --version)"
echo "G20: rustc $(rustc --version)"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"

echo "G20: gix-exit / no-celir scan"
meta="$(cargo metadata --format-version 1)"
if echo "$meta" | grep -E '"name":\s*"gix|"name":\s*"gitoxide|"name":\s*"gix-' >/dev/null; then
  echo "G20: forbidden gix/gitoxide edge" >&2
  exit 1
fi
if echo "$meta" | grep -Ei '"name":\s*"celir|"name":\s*"celir-' >/dev/null; then
  echo "G20: forbidden celir edge" >&2
  exit 1
fi

export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/workspace/target}"

echo "G20: build arks (release)"
cargo build -p arks --release

mkdir -p verify/gates/g20/out
export ARKS_G20_RECEIPT_OUT="${ARKS_G20_RECEIPT_OUT:-/workspace/verify/gates/g20/out/receipt.json}"
# Always prefer the release binary just built into CARGO_TARGET_DIR (compose may set
# target-cleanroom). G14 must honor ARKS_BIN so regression does not pick a stale debug.
export ARKS_BIN="${ARKS_BIN:-$CARGO_TARGET_DIR/release/arks}"
echo "G20: ARKS_BIN=$ARKS_BIN"

echo "G20: Fs-1 suite (verify/gates/g20)"
cargo test --manifest-path verify/gates/g20/Cargo.toml --release "$@"

echo "G20: G14 regression (historical M9)"
cargo test --manifest-path verify/gates/g14/Cargo.toml "$@"

echo "G20: PASS"
