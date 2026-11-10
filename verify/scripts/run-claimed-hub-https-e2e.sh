#!/usr/bin/env bash
# Claimed residual gate B3 — hub HTTPS forge remote e2e.
# SoT: .spec/sot/arks-claimed-residual-gates.sot.md
# MUST NOT invent PASS; product MUST NOT spawn gh.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

SOT="${ARKS_CLAIMED_RESIDUAL_SOT:-.spec/sot/arks-claimed-residual-gates.sot.md}"
echo "claimed-hub-https-e2e: tip=$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"

if [[ ! -f "$SOT" ]]; then
  echo "claimed-hub-https-e2e: SKIP — missing SoT $SOT"
  exit 0
fi
echo "claimed-hub-https-e2e: SoT present (design linked); B3 still Missing"

if [[ "${ARKS_DOD_STRICT:-0}" == "1" ]]; then
  echo "claimed-hub-https-e2e: FAIL STRICT — B3 hub HTTPS forge remote e2e not Claimed (no invent PASS)"
  exit 1
fi

echo "claimed-hub-https-e2e: SKIP/scaffold PASS — hub smoke/--local-tls ≠ Claimed"
exit 0
