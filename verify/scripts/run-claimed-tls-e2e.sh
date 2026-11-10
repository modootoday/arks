#!/usr/bin/env bash
# Claimed residual gate B9 — product in-process TLS terminate.
# SoT: .spec/sot/arks-claimed-residual-gates.sot.md
# MUST NOT invent PASS; ops reverse-proxy alone ≠ B9.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

SOT="${ARKS_CLAIMED_RESIDUAL_SOT:-.spec/sot/arks-claimed-residual-gates.sot.md}"
echo "claimed-tls-e2e: tip=$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"

if [[ ! -f "$SOT" ]]; then
  echo "claimed-tls-e2e: SKIP — missing SoT $SOT"
  exit 0
fi
echo "claimed-tls-e2e: SoT present (design linked); B9 still Missing"

if [[ "${ARKS_DOD_STRICT:-0}" == "1" ]]; then
  echo "claimed-tls-e2e: FAIL STRICT — B9 product in-process TLS not Claimed (no invent PASS)"
  exit 1
fi

echo "claimed-tls-e2e: SKIP/scaffold PASS — ops TLS front / hub --local-tls ≠ product TLS Claimed"
exit 0
