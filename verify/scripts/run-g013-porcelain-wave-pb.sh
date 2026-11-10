#!/usr/bin/env bash
# G13 — Wave Pb porcelain + G12/G7/G6 regression + gix-exit.
# Not production (production = M9).
set -euo pipefail

cd /workspace

if ! command -v git >/dev/null; then
  echo "G13: stock git missing in image" >&2
  exit 1
fi
echo "G13: stock git $(git --version)"
echo "G13: rustc $(rustc --version)"
echo "G13: CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-2}"

export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"

echo "G13: gix-exit scan"
if cargo metadata --format-version 1 --no-deps 2>/dev/null | grep -E '"name":\s*"(gix|gitoxide)' >/dev/null; then
  echo "G13: forbidden gix/gitoxide package name in workspace members" >&2
  exit 1
fi
meta="$(cargo metadata --format-version 1)"
if echo "$meta" | grep -E '"name":\s*"gix|"name":\s*"gitoxide|"name":\s*"gix-' >/dev/null; then
  echo "G13: forbidden gix/gitoxide edge in cargo metadata" >&2
  exit 1
fi
echo "G13: gix-exit OK"

echo "G13: build arks binary"
cargo build -p arks

echo "G13: Wave Pb porcelain suite (verify/gates/g013-porcelain-wave-pb)"
cargo test --manifest-path verify/gates/g013-porcelain-wave-pb/Cargo.toml "$@"

echo "G13: arks-remote + arks unit tests"
cargo test -p arks-remote "$@"
cargo test -p arks "$@"

echo "G13: G12 Wave Pa regression (verify/gates/g012-porcelain-wave-pa)"
cargo test --manifest-path verify/gates/g012-porcelain-wave-pa/Cargo.toml "$@"

echo "G13: G7 Wave H regression (verify/gates/g007-smart-http-client)"
cargo test --manifest-path verify/gates/g007-smart-http-client/Cargo.toml "$@"

echo "G13: G6 Wave L regression (verify/gates/g006-cli-wave-l)"
cargo test --manifest-path verify/gates/g006-cli-wave-l/Cargo.toml "$@"

echo "G13: G3/G4/G5/G2/G10/G1 regression"
cargo test --manifest-path verify/gates/g003-index-commit-roundtrip/Cargo.toml "$@"
cargo test --manifest-path verify/gates/g004-pack-idx-roundtrip/Cargo.toml "$@"
cargo test --manifest-path verify/gates/g005-protocol-offline-fixtures/Cargo.toml "$@"
cargo test -p arks-protocol "$@"
cargo test --manifest-path verify/gates/g002-refs-index-roundtrip/Cargo.toml "$@"
cargo test --manifest-path verify/gates/g010-store-facade/Cargo.toml "$@"
cargo test -p arks-object --test g001-loose-object-interop_oracle "$@"
cargo test --manifest-path verify/gates/g001-loose-object-interop/Cargo.toml "$@"

echo "G13: PASS"
