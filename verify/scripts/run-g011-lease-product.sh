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

echo "G11: gix-exit / no-celir scan"
if cargo metadata --format-version 1 --no-deps 2>/dev/null | grep -E '"name":\s*"(gix|gitoxide)' >/dev/null; then
  echo "G11: forbidden gix/gitoxide package name in workspace members" >&2
  exit 1
fi
# Optional path-deps to discrete-circuit appear in full `cargo metadata` whenever
# the sibling tree is mounted — parse-time only. Product honesty = default feature
# graph (same as G15/G20/G22): celir-index stays OFF.
if cargo tree -p arks-store --edges normal 2>/dev/null | grep -Ei 'celir' >/dev/null; then
  echo "G11: FAIL — default arks-store tree has celir edge" >&2
  cargo tree -p arks-store --edges normal >&2 || true
  exit 1
fi
echo "G11: gix-exit / default tree CELIR-free OK"

echo "G11: build arks binary"
cargo build -p arks

echo "G11: arks-lease unit tests (L1/L2/dual-path/lock-additive)"
cargo test -p arks-lease "$@"

echo "G11: lease-product suite (verify/gates/g11)"
cargo test --manifest-path verify/gates/g11/Cargo.toml "$@"

echo "G11: G14 CLI-parity regression (verify/gates/g14)"
cargo test --manifest-path verify/gates/g14/Cargo.toml "$@"

echo "G11: G7 remote-client regression (verify/gates/g7)"
cargo test --manifest-path verify/gates/g7/Cargo.toml "$@"

echo "G11: PASS"
