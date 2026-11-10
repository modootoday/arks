#!/usr/bin/env bash
# Re-evaluate G19 from an existing Docker G16 metrics.json (no rebuild).
set -euo pipefail
cd /workspace
METRICS="${ARKS_JOURNEY_METRICS_OUT:-/workspace/verify/gates/g016-journey-compare/out/metrics.json}"
if [[ ! -f "$METRICS" ]]; then
  echo "missing metrics: $METRICS" >&2
  exit 1
fi
mkdir -p verify/gates/g019-comparative-performance/out
export ARKS_G19_METRICS="$METRICS"
export ARKS_G19_RECEIPT_OUT="${ARKS_G19_RECEIPT_OUT:-/workspace/verify/gates/g019-comparative-performance/out/receipt.json}"
export ARKS_G19_REVISION="$(git -C /workspace rev-parse HEAD 2>/dev/null || echo unknown)"
export ARKS_G19_IMAGE_TAG="${ARKS_JOURNEY_IMAGE_TAG:-arks-verify-m1:0.1.0}"
export ARKS_G19_IMAGE_DIGEST="${ARKS_JOURNEY_IMAGE_DIGEST:-}"
export ARKS_G19_STOCK_GIT="$(git --version)"
python3 /workspace/verify/scripts/g019-eval.py
echo "G19: PASS (eval-only)"
