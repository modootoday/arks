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

echo "G6: Wave L argv suite (verify/gates/g006-cli-wave-l)"
cargo test --manifest-path verify/gates/g006-cli-wave-l/Cargo.toml "$@"

echo "G6: full G3 index-commit (verify/gates/g003-index-commit-roundtrip)"
cargo test --manifest-path verify/gates/g003-index-commit-roundtrip/Cargo.toml "$@"

echo "G6: G4 pack + G5 protocol + G2/G10/G1 regression"
cargo test --manifest-path verify/gates/g004-pack-idx-roundtrip/Cargo.toml "$@"
cargo test --manifest-path verify/gates/g005-protocol-offline-fixtures/Cargo.toml "$@"
cargo test -p arks-protocol "$@"
cargo test --manifest-path verify/gates/g002-refs-index-roundtrip/Cargo.toml "$@"
cargo test --manifest-path verify/gates/g010-store-facade/Cargo.toml "$@"
cargo test -p arks-object --test g001-loose-object-interop_oracle "$@"
cargo test --manifest-path verify/gates/g001-loose-object-interop/Cargo.toml "$@"
