#!/usr/bin/env bash
# G38 — FW-GPGSIG OpenPGP / X.509 / tag -s (R4 / S2)
set -euo pipefail
cd /workspace
if ! command -v git >/dev/null; then echo G38: stock git missing >&2; exit 1; fi
if ! command -v gpg >/dev/null; then echo G38: gpg missing >&2; exit 1; fi
echo "G38: stock git $(git --version)"
echo "G38: $(gpg --version | head -1)"
echo "G38: rustc $(rustc --version)"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
meta="$(cargo metadata --format-version 1)"
if echo "$meta" | grep -E '"name":[[:space:]]*"gix' >/dev/null; then echo G38: forbidden gix >&2; exit 1; fi
echo G38: gix-exit OK
echo G38: build arks
cargo build -p arks
echo G38: OpenPGP / tag suite
cargo test --manifest-path verify/gates/g38/Cargo.toml -- --nocapture
echo G38: PASS