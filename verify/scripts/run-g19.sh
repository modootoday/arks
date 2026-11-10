#!/usr/bin/env bash
# G19 — comparative performance pass/fail (≥20% faster on allow-list).
# SoT: interop-test-matrix G19; matrix: verify/matrix-g19.md; plan: 20261109043916
set -euo pipefail

cd /workspace

# Exclusive lock: overlapping G19 catalogs OOM Docker Desktop (exit 137).
mkdir -p verify/g19
G19_LOCK="verify/g19/.run.lock.dir"
if ! mkdir "$G19_LOCK" 2>/dev/null; then
  # Stale lock after SIGKILL (trap skipped): reclaim if holder file is older than 2h
  # or empty. Fresh holders write a timestamp owner file.
  if [[ -f "$G19_LOCK/owner" ]]; then
    age=$(( $(date +%s) - $(stat -c %Y "$G19_LOCK/owner" 2>/dev/null || echo 0) ))
    if [[ "$age" -gt 7200 ]]; then
      echo "G19: reclaiming stale lock (age=${age}s)"
      rm -rf "$G19_LOCK"
      mkdir "$G19_LOCK"
    else
      echo "G19: another run holds $G19_LOCK — refuse overlap (OOM risk; run one at a time)" >&2
      exit 75
    fi
  else
    echo "G19: reclaiming empty/stale lock dir"
    rm -rf "$G19_LOCK"
    mkdir "$G19_LOCK"
  fi
fi
echo "pid=$$ host=$(hostname) ts=$(date -Iseconds)" >"$G19_LOCK/owner"
trap 'rm -rf "$G19_LOCK" 2>/dev/null || true' EXIT

echo "G19: running G16 functional+timing baseline (iterations>=3, release)"
export ARKS_JOURNEY_ITERATIONS="${ARKS_JOURNEY_ITERATIONS:-3}"
if [[ "${ARKS_JOURNEY_ITERATIONS}" -lt 3 ]]; then
  echo "G19: ARKS_JOURNEY_ITERATIONS must be >=3 (got ${ARKS_JOURNEY_ITERATIONS})" >&2
  exit 1
fi
/bin/bash verify/scripts/run-g16.sh "$@"

METRICS="${ARKS_JOURNEY_METRICS_OUT:-/workspace/verify/g16/out/metrics.json}"
if [[ ! -f "$METRICS" ]]; then
  echo "G19: missing metrics: $METRICS" >&2
  exit 1
fi

mkdir -p verify/g19/out
RECEIPT="${ARKS_G19_RECEIPT_OUT:-/workspace/verify/g19/out/receipt.json}"
export ARKS_G19_RECEIPT_OUT="$RECEIPT"
export ARKS_G19_METRICS="$METRICS"
export ARKS_G19_REVISION="$(git -C /workspace rev-parse HEAD 2>/dev/null || echo unknown)"
export ARKS_G19_IMAGE_TAG="${ARKS_JOURNEY_IMAGE_TAG:-arks-verify-m1:0.1.0}"
export ARKS_G19_IMAGE_DIGEST="${ARKS_JOURNEY_IMAGE_DIGEST:-}"
export ARKS_G19_STOCK_GIT="$(git --version)"

echo "G19: evaluate Pf3 allow-list = all G16 journeys (arks_median <= 0.8 * git_median)"
# External evaluator avoids bash-heredoc races on bind-mounted Windows hosts.
python3 /workspace/verify/scripts/g19_eval.py

echo "G19: PASS"
