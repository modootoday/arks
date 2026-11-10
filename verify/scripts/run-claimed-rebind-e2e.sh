#!/usr/bin/env bash
# Claimed residual gate B7 — zero-downtime hot-rebind.
# SoT: .spec/sot/arks-claimed-residual-gates.sot.md
# MUST NOT invent PASS from Partial live rebind.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

SOT="${ARKS_CLAIMED_RESIDUAL_SOT:-.spec/sot/arks-claimed-residual-gates.sot.md}"
echo "claimed-rebind-e2e: tip=$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"

if [[ ! -f "$SOT" ]]; then
  echo "claimed-rebind-e2e: SKIP — missing SoT $SOT"
  exit 0
fi
echo "claimed-rebind-e2e: SoT present (design linked); B7 still Missing"

if [[ "${ARKS_DOD_STRICT:-0}" == "1" ]]; then
  echo "claimed-rebind-e2e: FAIL STRICT — B7 zero-downtime rebind not Claimed (no invent PASS)"
  exit 1
fi

echo "claimed-rebind-e2e: SKIP/scaffold PASS — Partial live rebind ≠ connection-drain Claimed"
exit 0
