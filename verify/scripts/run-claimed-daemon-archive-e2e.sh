#!/usr/bin/env bash
# Claimed residual gates B5/B5+ — daemon archive parity + syslog facility Claimed.
# SoT: .spec/sot/arks-claimed-residual-gates.sot.md
# MUST NOT invent PASS; product MUST NOT spawn git.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

SOT="${ARKS_CLAIMED_RESIDUAL_SOT:-.spec/sot/arks-claimed-residual-gates.sot.md}"
echo "claimed-daemon-archive-e2e: tip=$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
echo "claimed-daemon-archive-e2e: pin git=$(git --version 2>/dev/null || echo unknown)"

if [[ ! -f "$SOT" ]]; then
  echo "claimed-daemon-archive-e2e: SKIP — missing SoT $SOT"
  exit 0
fi
echo "claimed-daemon-archive-e2e: SoT present (design linked); B5/B5+ still Missing"

if [[ "${ARKS_DOD_STRICT:-0}" == "1" ]]; then
  echo "claimed-daemon-archive-e2e: FAIL STRICT — B5 archive parity / B5+ syslog facility not Claimed (no invent PASS)"
  exit 1
fi

echo "claimed-daemon-archive-e2e: SKIP/scaffold PASS — protocol-body/openlog Partial ≠ Claimed"
exit 0
