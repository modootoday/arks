#!/usr/bin/env bash
# G41 — Apache mod_dav dumb HTTP interop (O4).
# Expects compose services: g41-seed → g41-dav → this runner (ARKS_G41_DAV_URL).
set -euo pipefail
cd /workspace
if ! command -v git >/dev/null; then echo G41: stock git missing >&2; exit 1; fi
echo "G41: stock git $(git --version)"
echo "G41: rustc $(rustc --version)"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"
meta="$(cargo metadata --format-version 1)"
if echo "$meta" | grep -E '"name":[[:space:]]*"gix' >/dev/null; then echo G41: forbidden gix >&2; exit 1; fi
echo G41: gix-exit OK
mkdir -p verify/gates/g41/out
: "${ARKS_G41_DAV_URL:=http://g41-dav/repo.git/}"
export ARKS_G41_DAV_URL
echo "G41: peer $ARKS_G41_DAV_URL"
echo G41: build arks
cargo build -p arks
echo G41: Apache DAV suite
cargo test --manifest-path verify/gates/g41/Cargo.toml -- --nocapture
echo G41: PASS
echo "G41 PASS" > verify/gates/g41/out/summary.txt
