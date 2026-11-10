#!/usr/bin/env bash
# G4 — pack/idx local round-trip vs stock git, then G5 + G2/G10/G1 regression.
set -euo pipefail

cd /workspace

if ! command -v git >/dev/null; then
  echo "G4: stock git missing in image" >&2
  exit 1
fi
echo "G4: stock git $(git --version)"
echo "G4: rustc $(rustc --version)"
echo "G4: CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-2}"

export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"

echo "G4: pack/idx oracle (verify/gates/g4)"
cargo test --manifest-path verify/gates/g4/Cargo.toml "$@"

echo "G4: G5 protocol-offline (verify/gates/g5 + arks-protocol)"
cargo test --manifest-path verify/gates/g5/Cargo.toml "$@"
cargo test -p arks-protocol "$@"

echo "G4: G2/G10/G1 regression (M4 must not break refs/index/facade/loose-object interop)"
cargo test --manifest-path verify/gates/g2/Cargo.toml "$@"
cargo test --manifest-path verify/gates/g10/Cargo.toml "$@"
cargo test -p arks-object --test g1_oracle "$@"
cargo test --manifest-path verify/gates/g1/Cargo.toml "$@"
