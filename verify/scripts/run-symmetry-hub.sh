#!/usr/bin/env bash
# Alias → DoD pillar 3 (gh ↔ arks hub). Cite: sot:root/verify-cleanroom-symmetry-matrix
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
exec bash "$ROOT/verify/scripts/run-hub-gh-arks-matrix-e2e.sh" "$@"
