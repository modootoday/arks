#!/usr/bin/env bash
# Orchestrator for Claimed residual gates B1/B2/B3/B4/B5/B5+/B7/B9.
# SoT: .spec/sot/arks-claimed-residual-gates.sot.md
# Default SKIP/scaffold; ARKS_DOD_STRICT=1 fails closed until Claimed.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
SCRIPTS="$(cd "$(dirname "$0")" && pwd)"

echo "claimed-residual-dod: tip=$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
echo "claimed-residual-dod: strict=${ARKS_DOD_STRICT:-0}"

fail=0
run_one() {
  local name="$1"
  local script="$2"
  echo "claimed-residual-dod: --- $name ---"
  if ! /bin/bash "$script"; then
    echo "claimed-residual-dod: HARD FAIL — $name"
    fail=1
    if [[ "${ARKS_DOD_STRICT:-0}" == "1" ]]; then
      exit 1
    fi
  fi
}

run_one "ssh(B1/B2)" "$SCRIPTS/run-claimed-ssh-e2e.sh"
run_one "hub-https(B3)" "$SCRIPTS/run-claimed-hub-https-e2e.sh"
run_one "agent-llm(B4)" "$SCRIPTS/run-claimed-agent-llm-e2e.sh"
run_one "daemon-archive(B5/B5+)" "$SCRIPTS/run-claimed-daemon-archive-e2e.sh"
run_one "rebind(B7)" "$SCRIPTS/run-claimed-rebind-e2e.sh"
run_one "tls(B9)" "$SCRIPTS/run-claimed-tls-e2e.sh"

if [[ "$fail" -ne 0 ]]; then
  echo "claimed-residual-dod: FAIL (one or more slots)"
  exit 1
fi

echo "claimed-residual-dod: SKIP/scaffold orchestrator PASS — design linked; no invent Claimed"
exit 0
