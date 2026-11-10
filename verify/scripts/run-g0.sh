#!/usr/bin/env bash
# G0 — workspace library tests in clean-room (authoritative when green).
set -euo pipefail

cd /workspace

if ! command -v git >/dev/null; then
  echo "G0: stock git missing in image" >&2
  exit 1
fi
echo "G0: stock git $(git --version)"
echo "G0: rustc $(rustc --version)"
echo "G0: CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-2}"

export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
cargo test --workspace "$@"
