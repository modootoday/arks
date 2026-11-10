#!/usr/bin/env bash
# G31 — server protocol v2 (M25 / FW-SRV-V2)
set -euo pipefail
cd /workspace
if ! command -v git >/dev/null; then
  echo "G31: stock git missing" >&2
  exit 1
fi
echo "G31: stock git $(git --version)"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
echo "G31: build arks"
cargo build -p arks
echo "G31: protocol v2 suite"
cargo test --manifest-path verify/gates/g31/Cargo.toml "$@"
echo "G31: PASS"
