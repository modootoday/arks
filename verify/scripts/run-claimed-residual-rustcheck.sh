#!/usr/bin/env bash
# Advisory Rust hygiene for claimed-residual WIP (fmt + focused tests + check).
# SoT: .spec/sot/verify-directory-conventions.sot.md — scripts under verify/scripts/;
# compose image PATH; non-login bash. Host PowerShell is not the judge.
# Pattern mirrors run-fmt-check.sh: unset RUSTUP_TOOLCHAIN so image rustfmt resolves.
set -euo pipefail
cd /workspace
unset RUSTUP_TOOLCHAIN || true
echo "claimed-residual-rustcheck: rustc $(rustc --version)"
echo "claimed-residual-rustcheck: cargo $(cargo --version)"
rustup component add rustfmt 2>/dev/null || true
cargo fmt --all -- --check
cargo test -p arks-agent -p arks-server
cargo check -p arks
echo "claimed-residual-rustcheck: OK"
