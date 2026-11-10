#!/usr/bin/env bash
# Hub local multi-agent load runner (scaffold).
# SoT: .spec/sot/arks-hub-local-multi-agent-pr.sot.md
# MUST NOT invent PASS; MUST NOT spawn gh/git.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

SCENARIO="${ARKS_HUB_LOCAL_SCENARIO:-.spec/sot/arks-hub-local-multi-agent-pr.sot.md}"

echo "hub-local-multi-agent: tip=$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"

if [[ ! -f "$SCENARIO" ]]; then
  echo "hub-local-multi-agent: SKIP — missing scenario SoT $SCENARIO"
  exit 0
fi

echo "hub-local-multi-agent: scenario SoT present ($SCENARIO)"
echo "hub-local-multi-agent: TODO — local arks hub serve + N-agent HUB-MA-* runners not Claimed"
echo "hub-local-multi-agent: SKIP — scaffold only (no invent PASS)"
exit 0
