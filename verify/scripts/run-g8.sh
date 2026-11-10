#!/usr/bin/env bash
# G8 — live GitHub-class remote smoke: default-on when secrets present; honest SKIP when absent.
# ADR: .spec/adr/20261110220000-g8-live-forge-default-on-when-secrets.adr.md
# (supersedes adr:root/20261110151000)
set -euo pipefail

cd /workspace

mkdir -p verify/g8/out
RECEIPT="${ARKS_G8_RECEIPT_OUT:-/workspace/verify/g8/out/receipt.json}"
REVISION="$(git -C /workspace rev-parse HEAD 2>/dev/null || echo unknown)"
STOCK_GIT="$(git --version)"
IMAGE_TAG="${ARKS_JOURNEY_IMAGE_TAG:-arks-verify-m1:0.1.0}"

write_receipt() {
  local disposition="$1"
  local detail="$2"
  python3 - "$RECEIPT" "$disposition" "$detail" "$REVISION" "$STOCK_GIT" "$IMAGE_TAG" <<'PY'
import json, os, sys
out, disposition, detail, revision, stock_git, image_tag = sys.argv[1:7]
receipt = {
    "schema": "arks-g8-live-smoke/v1",
    "gate": "G8",
    "disposition": disposition,
    "detail": detail,
    "venue": "clean-room",
    "revision": revision,
    "stock_git": stock_git,
    "image_tag": image_tag,
    "secrets": {
        "ARKS_G8_LIVE": os.environ.get("ARKS_G8_LIVE", ""),
        "has_token": bool(os.environ.get("ARKS_G8_TOKEN") or os.environ.get("GITHUB_TOKEN")),
        "has_repo_url": bool(os.environ.get("ARKS_G8_REPO_URL")),
    },
    "adr": "adr:root/20261110220000",
}
with open(out, "w", encoding="utf-8") as f:
    json.dump(receipt, f, indent=2)
    f.write("\n")
print(f"G8: receipt written to {out} disposition={disposition}")
PY
}

if ! command -v git >/dev/null; then
  echo "G8: stock git missing in image" >&2
  exit 1
fi
echo "G8: stock git ${STOCK_GIT}"

LIVE_OVERRIDE="${ARKS_G8_LIVE:-}"
TOKEN="${ARKS_G8_TOKEN:-${GITHUB_TOKEN:-}}"
REPO_URL="${ARKS_G8_REPO_URL:-}"

if [[ -z "${TOKEN}" || -z "${REPO_URL}" ]]; then
  echo "G8: SKIP / refuse-closed — secrets absent (ADR 20261110220000)"
  echo "G8: set ARKS_G8_TOKEN (or GITHUB_TOKEN) + ARKS_G8_REPO_URL for default-on live smoke"
  write_receipt "skip-secrets-absent" "token/URL absent; G7 remains default offline remote proof; not a live PASS"
  echo "G8: SKIP OK"
  exit 0
fi

if [[ "${LIVE_OVERRIDE}" == "0" ]]; then
  echo "G8: SKIP — ARKS_G8_LIVE=0 force-off with secrets present (ADR 20261110220000)"
  write_receipt "skip-force-off" "secrets present but ARKS_G8_LIVE=0; operator force-off"
  echo "G8: SKIP OK"
  exit 0
fi

echo "G8: default-on live smoke (secrets present; ARKS_G8_LIVE=1 not required)"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"
cargo build -p arks --release
ARKS_BIN="$(find "${CARGO_TARGET_DIR:-target}/release" target/release -name arks -type f 2>/dev/null | head -n1 || true)"
if [[ -z "${ARKS_BIN}" || ! -x "${ARKS_BIN}" ]]; then
  echo "G8: arks binary missing after build" >&2
  exit 1
fi
echo "G8: ARKS_BIN=${ARKS_BIN}"

WORK="$(mktemp -d /tmp/arks-g8.XXXXXX)"
trap 'rm -rf "$WORK"' EXIT

# Scrub token from argv display; use header auth via URL rewrite for HTTPS.
# Prefer x-access-token style without echoing secrets.
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

CLONE_DIR="${WORK}/clone"
echo "G8: clone throwaway peer (URL redacted)"
"${ARKS_BIN}" clone "${AUTH_URL}" "${CLONE_DIR}"

BRANCH="arks-g8-smoke-$(date +%s)"
cd "${CLONE_DIR}"
echo "g8-smoke $(date -Iseconds)" > g8-smoke.txt
"${ARKS_BIN}" add g8-smoke.txt
"${ARKS_BIN}" -c user.name=arks-g8 -c user.email=arks-g8@example.com commit -m "g8 smoke"
"${ARKS_BIN}" push -u origin "HEAD:refs/heads/${BRANCH}"
"${ARKS_BIN}" fetch origin "${BRANCH}"
TIP="$("${ARKS_BIN}" rev-parse "FETCH_HEAD")"
echo "G8: live push/fetch tip ${TIP} on ${BRANCH}"

write_receipt "closed-implement" "live clone/push/fetch ok branch=${BRANCH} tip=${TIP}"
echo "G8: PASS (live)"
