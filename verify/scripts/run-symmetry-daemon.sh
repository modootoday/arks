#!/usr/bin/env bash
# Alias → DoD pillar 2 (git-daemon ↔ arks daemon). Cite: sot:root/verify-cleanroom-symmetry-matrix
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
exec bash "$ROOT/verify/scripts/run-daemon-git-arks-matrix-e2e.sh" "$@"
