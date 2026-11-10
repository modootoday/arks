#!/usr/bin/env bash
# G40 — S3 dumb HTTP push deletes clean-room twin (S6 symmetry)
set -euo pipefail
cd /workspace
if ! command -v git >/dev/null; then echo G40: stock git missing >&2; exit 1; fi
echo "G40: stock git $(git --version)"
echo "G40: rustc $(rustc --version)"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"
meta="$(cargo metadata --format-version 1)"
if echo "$meta" | grep -E '"name":[[:space:]]*"gix' >/dev/null; then echo G40: forbidden gix >&2; exit 1; fi
echo G40: gix-exit OK
mkdir -p verify/gates/g040-dumb-http-push-deletes/out
echo G40: build arks
cargo build -p arks
echo G40: dumb HTTP delete suite
cargo test --manifest-path verify/gates/g040-dumb-http-push-deletes/Cargo.toml -- --nocapture
echo G40: PASS
