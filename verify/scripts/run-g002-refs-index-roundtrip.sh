#!/usr/bin/env bash
# G2 — refs round-trip + index r/w vs stock git, then G10/G1 regression.
set -euo pipefail

cd /workspace

if ! command -v git >/dev/null; then
  echo "G2: stock git missing in image" >&2
  exit 1
fi
echo "G2: stock git $(git --version)"
echo "G2: rustc $(rustc --version)"
echo "G2: CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-2}"

export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"

echo "G2: refs + index oracle (verify/gates/g002-refs-index-roundtrip)"
cargo test --manifest-path verify/gates/g002-refs-index-roundtrip/Cargo.toml "$@"

echo "G2: G10/G1 regression (M3 must not break facade/loose-object interop)"
cargo test --manifest-path verify/gates/g010-store-facade/Cargo.toml "$@"
cargo test -p arks-object --test g001-loose-object-interop_oracle "$@"
cargo test --manifest-path verify/gates/g001-loose-object-interop/Cargo.toml "$@"