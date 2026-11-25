#!/usr/bin/env bash
# Hub local multi-agent scenarios (HUB-MA-*) against one local `arks serve` forge.
# SoT: .spec/sot/arks-hub-local-multi-agent-pr.sot.md
# Driver: verify/scripts/hub-local-multi-agent.py (agents only ever run `arks`).
# MUST NOT invent PASS; MUST NOT spawn gh/git.
#
# Env:
#   ARKS_BIN               arks binary under test (default: built release binary)
#   ARKS_HUB_MA_AGENTS     number of agent accounts (default 6)
#   ARKS_HUB_MA_SCENARIOS  comma list of scenario IDs (default: all with a runner)
#   ARKS_HUB_MA_OUT        output directory (default verify/.hub-local-multi-agent-out)
# A Docker run (/.dockerenv) also writes receipt.json into the output directory.
# Exit 0 only when every selected scenario passed; 1 otherwise.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

SCENARIO="${ARKS_HUB_LOCAL_SCENARIO:-.spec/sot/arks-hub-local-multi-agent-pr.sot.md}"
TIP="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
echo "hub-local-multi-agent: tip=${TIP}"

if [[ ! -f "$SCENARIO" ]]; then
  echo "hub-local-multi-agent: SKIP — missing scenario SoT $SCENARIO"
  exit 0
fi
command -v python3 >/dev/null || { echo "FAIL: python3 missing" >&2; exit 1; }

ARKS_BIN="${ARKS_BIN:-}"
if [[ -z "$ARKS_BIN" ]]; then
  export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"
  cargo build -p arks --release
  ARKS_BIN="${CARGO_TARGET_DIR:-target}/release/arks"
fi
[[ -x "$ARKS_BIN" ]] || { echo "FAIL: arks binary missing: $ARKS_BIN" >&2; exit 1; }
ARKS_BIN="$(cd "$(dirname "$ARKS_BIN")" && pwd)/$(basename "$ARKS_BIN")"

OUT="${ARKS_HUB_MA_OUT:-verify/.hub-local-multi-agent-out}"
rm -rf "$OUT"
mkdir -p "$OUT"
OUT="$(cd "$OUT" && pwd)"
WORK="$(mktemp -d /tmp/arks-hub-ma.XXXXXX)"
trap 'rm -rf "$WORK"' EXIT

set +e
python3 -I verify/scripts/hub-local-multi-agent.py "$ARKS_BIN" "$WORK" "$OUT" 2>&1 | tee "$OUT/run.log"
rc=${PIPESTATUS[0]}
set -e
echo "$rc" >"$OUT/status.txt"

venue="host-advisory"
[[ -f /.dockerenv ]] && venue="docker-dod-hub-matrix"
python3 - "$OUT" "$TIP" "$venue" "${ARKS_JOURNEY_IMAGE_TAG:-arks-verify-m1:0.1.0}" "$rc" <<'PY'
import json, sys, time
out, tip, venue, image, rc = sys.argv[1:]
try:
    summary = json.load(open(f"{out}/summary.json"))
except (OSError, ValueError):
    summary = {}
receipt = {
    "schema": "arks-hub-local-multi-agent/v1",
    "sot": "sot:root/arks-hub-local-multi-agent-pr",
    "venue": venue,
    "tip": tip,
    "image_tag": image if venue.startswith("docker") else None,
    "exit": int(rc),
    "agents": summary.get("agents"),
    "scenarios": {k: v["status"] for k, v in summary.get("scenarios", {}).items()},
    "note": "Docker receipt carries Claimed; a host run is advisory.",
    "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
}
with open(f"{out}/receipt.json", "w", encoding="utf-8") as f:
    json.dump(receipt, f, indent=2)
    f.write("\n")
PY

if [[ "$rc" -ne 0 ]]; then
  echo "hub-local-multi-agent: FAIL (see $OUT)" >&2
  exit 1
fi
echo "hub-local-multi-agent: all selected scenarios PASS ($venue; receipt $OUT/receipt.json)"
