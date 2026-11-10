#!/usr/bin/env bash
# Fail-closed operator path for live G8 / G8-live-scale remasure (O1).
#
# Probes Environment `g008-live-forge-smoke-live` secret *names* when `gh` is available, runs
# structural readiness, then exercises harness SKIP when secrets are absent.
# Never invents a live latency PASS. Exit 0 on honest secrets-block; exit 2
# when structural readiness fails.
#
# Cite: .spec/research/20261110205403-optional-residuals-o-ladder-o1-o-final.research.md
#       .spec/adr/20261110220000, .spec/adr/20261110221000

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

OUT_DIR="${ARKS_G8_OPERATOR_OUT:-${REPO_ROOT}/verify/gates/g008-live-forge-smoke/out/operator-path}"
mkdir -p "$OUT_DIR"

echo "## G8 live operator path (fail-closed)"
echo
echo "This path documents secrets presence and harness SKIP honesty."
echo "It does **not** invent G8 live PASS or scale medians."
echo

SECRET_COUNT="unknown"
SECRET_NAMES=""
if command -v gh >/dev/null 2>&1; then
  if gh api repos/modootoday/arks/environments/g008-live-forge-smoke-live/secrets \
    --jq '{total_count,names:[.secrets[].name]}' \
    >"$OUT_DIR/g008-live-forge-smoke-live-secrets-probe.json" 2>"$OUT_DIR/g008-live-forge-smoke-live-secrets-probe.err"; then
    SECRET_COUNT="$(python3 - <<'PY' "$OUT_DIR/g008-live-forge-smoke-live-secrets-probe.json"
import json,sys
print(json.load(open(sys.argv[1]))["total_count"])
PY
)"
    SECRET_NAMES="$(python3 - <<'PY' "$OUT_DIR/g008-live-forge-smoke-live-secrets-probe.json"
import json,sys
print(",".join(json.load(open(sys.argv[1])).get("names") or []) or "(none)")
PY
)"
    echo "- Environment \`g008-live-forge-smoke-live\` secret count (names only): **${SECRET_COUNT}**"
    echo "- Secret names: \`${SECRET_NAMES}\`"
  else
    echo "- Environment probe: unavailable (gh API error; see operator-path err log)"
    SECRET_COUNT="probe-failed"
  fi
else
  echo "- Environment probe: \`gh\` not installed; relying on process env only"
fi
echo

HAS_TOKEN=0
HAS_URL=0
if [[ -n "${ARKS_G8_TOKEN:-${GITHUB_TOKEN:-}}" ]]; then HAS_TOKEN=1; fi
if [[ -n "${ARKS_G8_REPO_URL:-}" ]]; then HAS_URL=1; fi
echo "- Process env token present: ${HAS_TOKEN}"
echo "- Process env ARKS_G8_REPO_URL present: ${HAS_URL}"
echo

echo "### Structural readiness"
bash scripts/forge-scale-readiness.sh | tee "$OUT_DIR/readiness.txt"
echo

if [[ "$HAS_TOKEN" -eq 1 && "$HAS_URL" -eq 1 && "${ARKS_G8_LIVE:-}" != "0" ]]; then
  echo "### Live path available (operator secrets in process env)"
  echo "Run compose \`g008-live-forge-smoke\` / \`g008-live-scale\` and attach real receipts under \`.spec/evidence/\`."
  echo "This script does not auto-fire live forge calls (sole-runner honesty)."
  cat >"$OUT_DIR/operator-disposition.json" <<EOF
{"schema":"arks-g008-live-forge-smoke-operator-path/v1","disposition":"secrets-present-manual-live","secret_count":"${SECRET_COUNT}","has_token":true,"has_repo_url":true,"live_pass":false,"note":"operator must run harness and attach receipts; this path does not invent PASS"}
EOF
  echo
  echo "g008-live-forge-smoke-live-operator-path: READY (secrets present; live PASS still receipt-only)"
  exit 0
fi

echo "### Secrets hard block — exercising honest SKIP harness"
export ARKS_G8_TOKEN="${ARKS_G8_TOKEN:-}"
export ARKS_G8_REPO_URL="${ARKS_G8_REPO_URL:-}"
export GITHUB_TOKEN="${GITHUB_TOKEN:-}"
export ARKS_G8_RECEIPT_OUT="${OUT_DIR}/g008-live-forge-smoke-receipt.json"
export ARKS_G8_SCALE_OUT="${OUT_DIR}/g008-live-scale"
mkdir -p "$ARKS_G8_SCALE_OUT"
bash verify/scripts/run-g008-live-forge-smoke.sh | tee "$OUT_DIR/run-g008-live-forge-smoke.log"
bash verify/scripts/run-g008-live-scale.sh | tee "$OUT_DIR/run-g008-live-scale.log"

echo
echo "### Local-forge wire substitute (optional; ≠ GitHub live PASS)"
echo "When secrets remain absent, prove HTTPS git wire on loopback via:"
echo "  docker compose -f verify/docker-compose.yml run --rm g008-local-forge"
echo "  # or: bash verify/scripts/run-g008-local-forge.sh"
echo "Disposition \`claimed-wire\` must never be rewritten as GitHub live PASS."

cat >"$OUT_DIR/operator-disposition.json" <<EOF
{"schema":"arks-g008-live-forge-smoke-operator-path/v1","disposition":"secrets-block","secret_count":"${SECRET_COUNT}","has_token":false,"has_repo_url":false,"live_pass":false,"local_forge_substitute":"g008-local-forge","note":"Environment g008-live-forge-smoke-live empty or process env absent; SKIP receipts only; not a live PASS; local-forge wire ≠ GitHub remasure"}
EOF

echo
echo "g008-live-forge-smoke-live-operator-path: SECRETS-BLOCK (hard block; SKIP ≠ PASS)"
exit 0
