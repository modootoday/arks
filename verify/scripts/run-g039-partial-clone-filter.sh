#!/usr/bin/env bash
# G39 — R3 blob:none clean-room twin (S6 symmetry)
set -euo pipefail
cd /workspace
if ! command -v git >/dev/null; then echo G39: stock git missing >&2; exit 1; fi
echo "G39: stock git $(git --version)"
echo "G39: rustc $(rustc --version)"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"
meta="$(cargo metadata --format-version 1)"
if echo "$meta" | grep -E '"name":[[:space:]]*"gix' >/dev/null; then echo G39: forbidden gix >&2; exit 1; fi
echo G39: gix-exit OK
mkdir -p verify/gates/g39/out
echo G39: build arks
cargo build -p arks
echo G39: blob:none suite
cargo test --manifest-path verify/gates/g39/Cargo.toml -- --nocapture
echo G39: PASS
