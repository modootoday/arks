#!/usr/bin/env bash
# G8+ live GitHub forge scale harness — measure when secrets present; honest no-measurement when absent.
# ADR: .spec/adr/20261110221000-g8-live-github-scale-claimed-harness.adr.md
# (supersedes adr:root/20261110161000; secrets policy aligned with adr:root/20261110220000)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "${ROOT}"

OUT_DIR="${ARKS_G8_SCALE_OUT:-${ROOT}/verify/g8-live-scale/out}"
mkdir -p "${OUT_DIR}"
RECEIPT="${OUT_DIR}/receipt.json"
SAMPLES="${OUT_DIR}/samples.json"

write_receipt() {
  local disposition="$1"
  local detail="$2"
  python3 - "$RECEIPT" "$disposition" "$detail" <<'PY'
import json, os, sys, time
out, disposition, detail = sys.argv[1:4]
receipt = {
    "schema": "arks-g8-live-scale/v1",
    "gate": "G8-live-scale",
    "disposition": disposition,
    "detail": detail,
    "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
    "adr": "adr:root/20261110221000",
    "secrets": {
        "ARKS_G8_LIVE": os.environ.get("ARKS_G8_LIVE", ""),
        "ARKS_G8_SCALE_MEASURE": os.environ.get("ARKS_G8_SCALE_MEASURE", ""),
        "has_token": bool(os.environ.get("ARKS_G8_TOKEN") or os.environ.get("GITHUB_TOKEN")),
        "has_repo_url": bool(os.environ.get("ARKS_G8_REPO_URL")),
    },
}
with open(out, "w", encoding="utf-8") as f:
    json.dump(receipt, f, indent=2)
    f.write("\n")
print(f"G8-live-scale: receipt -> {out} ({disposition})")
PY
}

TOKEN="${ARKS_G8_TOKEN:-${GITHUB_TOKEN:-}}"
REPO_URL="${ARKS_G8_REPO_URL:-}"
MEASURE_OVERRIDE="${ARKS_G8_SCALE_MEASURE:-}"
SAMPLES_N="${ARKS_G8_SCALE_SAMPLES:-5}"

if [[ -z "${TOKEN}" || -z "${REPO_URL}" ]]; then
  echo "G8-live-scale: SKIP / no-measurement — secrets absent (ADR 20261110221000)"
  echo "G8-live-scale: set ARKS_G8_TOKEN (or GITHUB_TOKEN) + ARKS_G8_REPO_URL for default-on samples"
  write_receipt "skip-no-measurement" "harness Claimed ready; no live scale samples without secrets; not a product SLO"
  echo '{"samples":[]}' > "${SAMPLES}"
  echo "G8-live-scale: SKIP OK"
  exit 0
fi

if [[ "${MEASURE_OVERRIDE}" == "0" ]]; then
  echo "G8-live-scale: SKIP — ARKS_G8_SCALE_MEASURE=0 force-off with secrets present"
  write_receipt "skip-force-off" "secrets present but ARKS_G8_SCALE_MEASURE=0; operator force-off"
  echo '{"samples":[]}' > "${SAMPLES}"
  echo "G8-live-scale: SKIP OK"
  exit 0
fi

if ! command -v git >/dev/null; then
  echo "G8-live-scale: stock git required for live samples" >&2
  exit 1
fi

AUTH_URL="$(python3 - "$REPO_URL" "$TOKEN" <<'PY'
import sys, urllib.parse
url, token = sys.argv[1], sys.argv[2]
u = urllib.parse.urlsplit(url)
netloc = f"x-access-token:{urllib.parse.quote(token, safe='')}@{u.hostname}"
if u.port:
    netloc += f":{u.port}"
print(urllib.parse.urlunsplit((u.scheme, netloc, u.path, u.query, u.fragment)))
PY
)"

echo "G8-live-scale: default-on live samples (secrets present; ARKS_G8_LIVE=1 not required) n=${SAMPLES_N}"
python3 - "$SAMPLES" "$SAMPLES_N" "$AUTH_URL" <<'PY'
import json, subprocess, sys, time
out, n_s, auth_url = sys.argv[1], int(sys.argv[2]), sys.argv[3]
samples = []
for i in range(n_s):
    t0 = time.perf_counter()
    subprocess.run(
        ["git", "ls-remote", auth_url, "HEAD"],
        check=True,
        capture_output=True,
        text=True,
    )
    dt_ms = (time.perf_counter() - t0) * 1000.0
    samples.append({"i": i, "op": "ls-remote-HEAD", "wall_ms": round(dt_ms, 3)})
payload = {"samples": samples, "op": "ls-remote-HEAD", "n": n_s}
with open(out, "w", encoding="utf-8") as f:
    json.dump(payload, f, indent=2)
    f.write("\n")
print(f"G8-live-scale: wrote {out} ({len(samples)} samples)")
PY

write_receipt "claimed-measurement" "live ls-remote samples recorded locally; not a product SLO"
echo "G8-live-scale: MEASUREMENT OK (operator-local timings only)"
