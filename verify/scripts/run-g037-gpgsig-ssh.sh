#!/usr/bin/env bash
# G37 — FW-GPGSIG SSH commit -S / verify-commit
set -euo pipefail
cd /workspace
if ! command -v git >/dev/null; then echo G37: stock git missing >&2; exit 1; fi
if ! command -v ssh-keygen >/dev/null; then echo G37: ssh-keygen missing >&2; exit 1; fi
echo "G37: stock git $(git --version)"
echo "G37: $(ssh -V 2>&1)"
echo "G37: rustc $(rustc --version)"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
meta="$(cargo metadata --format-version 1)"
if echo "$meta" | grep -E '"name":[[:space:]]*"gix' >/dev/null; then echo G37: forbidden gix >&2; exit 1; fi
echo G37: gix-exit OK
echo G37: build arks
cargo build -p arks
echo G37: SSH suite
cargo test --manifest-path verify/gates/g037-gpgsig-ssh/Cargo.toml -- --nocapture
echo G37: PASS
