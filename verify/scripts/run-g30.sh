#!/usr/bin/env bash
# G30 — server SSH command-mode / duplex upload-pack (M25 / FW-SRV-SSH)
set -euo pipefail
cd /workspace
if ! command -v git >/dev/null; then
  echo "G30: stock git missing" >&2
  exit 1
fi
echo "G30: stock git $(git --version)"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
echo "G30: build arks"
cargo build -p arks
echo "G30: SSH command-mode suite"
cargo test --manifest-path verify/g30/Cargo.toml "$@"
echo "G30: PASS"
