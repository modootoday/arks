#!/usr/bin/env bash
# Report secrets-gated live forge scale (G8 / G8-live-scale) dry-run readiness.
#
# Exit 0 when structural wiring is consistent with the Claimed harness model
# (matrix + scripts + compose + workflow + receipt schema). Never claims a
# live forge latency PASS or G8 Claimed remasure. Live samples remain
# secrets + receipt only.
#
# Cite: .spec/adr/20261110220000-g8-live-forge-default-on-when-secrets.adr.md
#       .spec/adr/20261110221000-g8-live-github-scale-claimed-harness.adr.md
#       .spec/research/20261110095142-product-completion-goal-ladder.research.md

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

echo "## Forge-scale readiness (dry-run)"
echo
echo "This report is **ops readiness**, not a live forge latency PASS."
echo "G8 functional smoke and G8-live-scale samples still need secrets + receipts."
echo "L5 Done ≠ G8 Claimed live remasure ≠ product SLO."
echo

missing=0
require_file() {
  local path="$1"
  local label="$2"
  if [[ -f "$path" ]]; then
    echo "- OK  \`$path\` — $label"
  else
    echo "- MISSING \`$path\` — $label"
    missing=$((missing + 1))
  fi
}

echo "### Structural files"
require_file "verify/scripts/run-g8.sh" "G8 live smoke entrypoint"
require_file "verify/scripts/run-g8-live-scale.sh" "G8-live-scale entrypoint"
require_file "verify/matrices/matrix-g8-live-scale.md" "scale matrix / honesty"
require_file "verify/docker-compose.yml" "compose services g8 / g8-live-scale"
require_file ".github/workflows/g8-forge-scale.yml" "workflow_dispatch dry-run + secrets-gated live"
require_file "docs/forge-scale.md" "maintainer Pages"
require_file ".spec/adr/20261110220000-g8-live-forge-default-on-when-secrets.adr.md" "G8 secrets policy ADR"
require_file ".spec/adr/20261110221000-g8-live-github-scale-claimed-harness.adr.md" "scale harness ADR"
echo

echo "### Compose services (names only)"
if grep -qE '^[[:space:]]*g8:[[:space:]]*$' verify/docker-compose.yml \
  && grep -qE '^[[:space:]]*g8-live-scale:[[:space:]]*$' verify/docker-compose.yml; then
  echo "- OK  services \`g8\` and \`g8-live-scale\` present"
else
  echo "- MISSING compose service \`g8\` and/or \`g8-live-scale\`"
  missing=$((missing + 1))
fi
echo

echo "### Expected secrets / Environment (YAML cannot invent values)"
echo "- Environment: \`g8-live\` (https://github.com/modootoday/arks/settings/environments)"
echo "- \`ARKS_G8_TOKEN\` or \`GITHUB_TOKEN\` — throwaway PAT; scrub from logs"
echo "- \`ARKS_G8_REPO_URL\` — HTTPS URL of a throwaway peer repo"
echo "- Optional force-off: \`ARKS_G8_LIVE=0\` (smoke), \`ARKS_G8_SCALE_MEASURE=0\` (scale)"
echo "- Absent secrets ⇒ honest SKIP / no-measurement receipt (exit 0); **not** a PASS"
echo

echo "### Receipt schemas (operator-local)"
echo "- G8 smoke: \`arks-g8-live-smoke/v1\` → \`verify/gates/g8/out/receipt.json\`"
echo "- G8 scale: \`arks-g8-live-scale/v1\` → \`verify/gates/g8-live-scale/out/receipt.json\`"
echo "- Samples (scale only): \`samples.json\` durations; never tokens"
echo "- Attach receipts under \`.spec/evidence/\` only when real samples exist;"
echo "  do **not** invent medians or product SLOs from empty SKIP runs"
echo

echo "### Workflow triggers"
echo "- \`workflow_dispatch\` dry-run (default): this report + harness SKIP paths —"
echo "  **no** live forge calls, **no** invented latency PASS."
echo "- \`workflow_dispatch\` live (\`dry_run=false\`): Environment \`g8-live\`;"
echo "  harness measures only when Environment secrets are present."
echo

echo "### How a live G8 / scale claim would be earned (receipt-only)"
echo "1. Configure Environment \`g8-live\` secrets (token + throwaway repo URL)."
echo "2. Run smoke: \`bash verify/scripts/run-g8.sh\` (Docker compose \`g8\` preferred)."
echo "3. Run scale: \`bash verify/scripts/run-g8-live-scale.sh\` (compose \`g8-live-scale\`)."
echo "4. Keep sole-runner honesty — do not stampede G14–G28 / G19 remasure."
echo "5. File evidence with real receipts only; residual R5 / ladder L5 still"
echo "   separate **harness Claimed** from **live remasure PASS** / product SLO."
echo

echo "### What still blocks Claimed live remasure / product SLO"
echo "- Missing Environment secrets (token and/or URL)"
echo "- Operator force-off env vars"
echo "- No attached receipt with real samples"
echo "- Public product SLO governance (never invent from operator-local timings)"
echo

if [[ "$missing" -ne 0 ]]; then
  echo "### Summary"
  echo "- Structural gaps: $missing"
  echo "- Live forge latency: **not claimed**"
  echo
  echo "forge-scale-readiness: FAIL (structural)"
  exit 1
fi

echo "### Summary"
echo "- Harness model: G8 smoke + G8-live-scale (Claimed ready; secrets-gated)"
echo "- Environment model: \`g8-live\` (secrets operator-owned)"
echo "- Live forge latency / G8 remasure PASS: **not claimed** by this dry-run"
echo
echo "forge-scale-readiness: OK (structural)"
