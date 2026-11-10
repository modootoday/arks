#!/usr/bin/env bash
# Orchestrate three clean-room symmetry DoD pillars (halt on hard FAIL).
# Cite: sot:root/verify-cleanroom-symmetry-matrix
# Smoke/SKIP is not FAIL. Strict: ARKS_DOD_STRICT=1.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
echo "=== dod pillar 1: git ↔ arks CLI ==="
bash "$ROOT/verify/scripts/run-symmetry-git.sh"
echo "=== dod pillar 2: git-daemon ↔ arks daemon ==="
bash "$ROOT/verify/scripts/run-symmetry-daemon.sh"
echo "=== dod pillar 3: gh ↔ arks hub ==="
bash "$ROOT/verify/scripts/run-symmetry-hub.sh"
echo "=== dod symmetry: all pillars completed (smoke/SKIP allowed unless STRICT) ==="
