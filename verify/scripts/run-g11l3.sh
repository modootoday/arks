#!/usr/bin/env bash
# G11-L3 — M23 HighConcurrency claimed-shape stress (G11 expansion).
# Dual-path: GitCompat L1/L2 regress + L3 pipeline stress. No CELIR. No shared-index N-way.
set -euo pipefail

cd /workspace

if ! command -v git >/dev/null; then
  echo "G11-L3: stock git missing in image" >&2
  exit 1
fi
echo "G11-L3: stock git $(git --version)"
echo "G11-L3: rustc $(rustc --version)"
echo "G11-L3: CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-2}"

export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"

echo "G11-L3: gix-exit / no-celir scan"
if cargo metadata --format-version 1 --no-deps 2>/dev/null | grep -E '"name":\s*"(gix|gitoxide)' >/dev/null; then
  echo "G11-L3: forbidden gix/gitoxide package name in workspace members" >&2
  exit 1
fi
meta="$(cargo metadata --format-version 1)"
if echo "$meta" | grep -E '"name":\s*"gix|"name":\s*"gitoxide|"name":\s*"gix-' >/dev/null; then
  echo "G11-L3: forbidden gix/gitoxide edge in cargo metadata" >&2
  exit 1
fi
if echo "$meta" | grep -Ei '"name":\s*"celir|"name":\s*"celir-' >/dev/null; then
  echo "G11-L3: forbidden celir edge in cargo metadata" >&2
  exit 1
fi
echo "G11-L3: gix-exit / no-celir OK"

echo "G11-L3: arks-lease unit + pipeline stress (L1/L2 + N=32/N=128 claimed shape)"
cargo test -p arks-lease "$@"

echo "G11-L3: lease-product suite regress (verify/gates/g11)"
cargo test --manifest-path verify/gates/g11/Cargo.toml "$@"

echo "G11-L3: PASS"
