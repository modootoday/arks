#!/usr/bin/env bash
# Alias → DoD pillar 1 (git ↔ arks CLI). Cite: sot:root/verify-cleanroom-symmetry-matrix
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
exec bash "$ROOT/verify/scripts/run-cli-git-arks-matrix-e2e.sh" "$@"
