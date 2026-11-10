#!/usr/bin/env bash
# Claimed residual gates B1/B2 — OpenSSH-exact cipher + SSH git e2e.
# SoT: .spec/sot/arks-claimed-residual-gates.sot.md
# MUST NOT invent PASS; product MUST NOT spawn ssh/git/gh.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

SOT="${ARKS_CLAIMED_RESIDUAL_SOT:-.spec/sot/arks-claimed-residual-gates.sot.md}"
echo "claimed-ssh-e2e: tip=$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
echo "claimed-ssh-e2e: pin git=$(git --version 2>/dev/null || echo unknown)"
echo "claimed-ssh-e2e: openssh=$(ssh -V 2>&1 || echo unknown)"

if [[ ! -f "$SOT" ]]; then
  echo "claimed-ssh-e2e: SKIP — missing SoT $SOT"
  exit 0
fi
echo "claimed-ssh-e2e: SoT present (design linked); B1/B2 still Missing"

if [[ "${ARKS_DOD_STRICT:-0}" == "1" ]]; then
  echo "claimed-ssh-e2e: FAIL STRICT — B1 OpenSSH-exact + B2 SSH git e2e not Claimed (no invent PASS)"
  exit 1
fi

echo "claimed-ssh-e2e: SKIP/scaffold PASS — Partial smokes ≠ Claimed; wait for evidence"
exit 0
