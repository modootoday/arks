#!/usr/bin/env bash
# G8 local-forge wire substitute — prove clone/push/fetch against a loopback
# arks smart-HTTP peer when GitHub Environment secrets are absent.
#
# Honesty: disposition is local-forge-wire / claimed-wire only.
# This is NOT a GitHub-hosted G8 live PASS and NOT a forge scale SLO.
#
# Cite: .spec/adr/20261110220000, .spec/adr/20261110221000
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "${ROOT}"

mkdir -p verify/gates/g008-local-forge/out
RECEIPT="${ARKS_G8_LOCAL_FORGE_OUT:-${ROOT}/verify/gates/g008-local-forge/out/receipt.json}"
REVISION="$(git -C "${ROOT}" rev-parse HEAD 2>/dev/null || echo unknown)"
STOCK_GIT="$(git --version 2>/dev/null || echo missing)"
IMAGE_TAG="${ARKS_JOURNEY_IMAGE_TAG:-arks-verify-m1:0.1.0}"

write_receipt() {
  local disposition="$1"
  local detail="$2"
  python3 - "$RECEIPT" "$disposition" "$detail" "$REVISION" "$STOCK_GIT" "$IMAGE_TAG" <<'PY'
import json, sys, time
out, disposition, detail, revision, stock_git, image_tag = sys.argv[1:7]
receipt = {
    "schema": "arks-g008-local-forge-wire/v1",
    "gate": "G8-local-forge",
    "disposition": disposition,
    "detail": detail,
    "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
    "venue": "loopback-smart-http",
    "revision": revision,
    "stock_git": stock_git,
    "image_tag": image_tag,
    "github_live_pass": False,
    "product_slo": False,
    "adr": "adr:root/20261110220000",
    "note": "Proves HTTPS git wire on local peer only; never invents GitHub Environment live PASS",
}
with open(out, "w", encoding="utf-8") as f:
    json.dump(receipt, f, indent=2)
    f.write("\n")
print(f"G8-local-forge: receipt -> {out} ({disposition})")
PY
}

if ! command -v git >/dev/null; then
  echo "G8-local-forge: stock git missing" >&2
  exit 1
fi
echo "G8-local-forge: stock git ${STOCK_GIT}"
echo "G8-local-forge: wire substitute (≠ GitHub live PASS)"

export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"
cargo build -p arks --release
ARKS_BIN="$(find "${CARGO_TARGET_DIR:-target}/release" target/release -name arks -type f 2>/dev/null | head -n1 || true)"
if [[ -z "${ARKS_BIN}" || ! -x "${ARKS_BIN}" ]]; then
  echo "G8-local-forge: arks binary missing after build" >&2
  exit 1
fi
echo "G8-local-forge: ARKS_BIN=${ARKS_BIN}"

WORK="$(mktemp -d /tmp/arks-g008-local-forge.XXXXXX)"
SERVER_PID=""
cleanup() {
  if [[ -n "${SERVER_PID}" ]] && kill -0 "${SERVER_PID}" 2>/dev/null; then
    kill "${SERVER_PID}" 2>/dev/null || true
    wait "${SERVER_PID}" 2>/dev/null || true
  fi
  rm -rf "${WORK}"
}
trap cleanup EXIT

BARE="${WORK}/peer.git"
SEED="${WORK}/seed"
CLONE="${WORK}/clone"
mkdir -p "${SEED}"
git -C "${SEED}" init -q
git -C "${SEED}" -c user.name=g008-live-forge-smoke-local -c user.email=g008-live-forge-smoke-local@example.com \
  commit --allow-empty -m "g008-local-forge seed" -q
git clone --bare -q "${SEED}" "${BARE}"

# Pick a free loopback port.
PORT="$(python3 - <<'PY'
import socket
s = socket.socket()
s.bind(("127.0.0.1", 0))
print(s.getsockname()[1])
s.close()
PY
)"
LISTEN="127.0.0.1:${PORT}"
URL="http://${LISTEN}/"

"${ARKS_BIN}" serve-http --listen "${LISTEN}" "${BARE}" \
  >"${WORK}/serve-http.stdout" 2>"${WORK}/serve-http.stderr" &
SERVER_PID=$!

# Wait until the listener accepts connections.
python3 - "$PORT" <<'PY'
import socket, sys, time
port = int(sys.argv[1])
deadline = time.time() + 20
last = None
while time.time() < deadline:
    s = socket.socket()
    s.settimeout(0.5)
    try:
        s.connect(("127.0.0.1", port))
        s.close()
        sys.exit(0)
    except OSError as e:
        last = e
        time.sleep(0.1)
print(f"serve-http not ready: {last}", file=sys.stderr)
sys.exit(1)
PY

BRANCH="arks-g008-live-forge-smoke-local-$(date +%s)"
echo "G8-local-forge: clone ${URL} (loopback)"
"${ARKS_BIN}" clone "${URL}" "${CLONE}"
cd "${CLONE}"
export GIT_AUTHOR_NAME=g008-live-forge-smoke-local GIT_AUTHOR_EMAIL=g008-live-forge-smoke-local@example.com
export GIT_COMMITTER_NAME=g008-live-forge-smoke-local GIT_COMMITTER_EMAIL=g008-live-forge-smoke-local@example.com
# Prefer env identity; arks may not honor stock-git `-c` globals.
git config user.name g008-live-forge-smoke-local
git config user.email g008-live-forge-smoke-local@example.com
echo "g008-local-forge $(date -Iseconds)" > g008-local-forge.txt
"${ARKS_BIN}" add g008-local-forge.txt
"${ARKS_BIN}" commit -m "g008-live-forge-smoke local forge wire"
# arks push does not Claim src:dst refspecs; create named branch then push.
"${ARKS_BIN}" branch "${BRANCH}"
"${ARKS_BIN}" push origin "${BRANCH}"
"${ARKS_BIN}" fetch origin "${BRANCH}"
TIP="$("${ARKS_BIN}" rev-parse "refs/remotes/origin/${BRANCH}")"
echo "G8-local-forge: push/fetch tip ${TIP} on ${BRANCH}"

# Optional scale-shaped samples against the same peer (still not product SLO).
SAMPLES_N="${ARKS_G8_LOCAL_FORGE_SAMPLES:-3}"
SAMPLES_OUT="${ROOT}/verify/gates/g008-local-forge/out/samples.json"
python3 - "$SAMPLES_OUT" "$SAMPLES_N" "$URL" <<'PY'
import json, subprocess, sys, time
out, n_s, url = sys.argv[1], int(sys.argv[2]), sys.argv[3]
samples = []
for i in range(n_s):
    t0 = time.perf_counter()
    subprocess.run(["git", "ls-remote", url, "HEAD"], check=True, capture_output=True, text=True)
    samples.append({"i": i, "op": "ls-remote-HEAD", "wall_ms": round((time.perf_counter() - t0) * 1000.0, 3)})
with open(out, "w", encoding="utf-8") as f:
    json.dump({"samples": samples, "op": "ls-remote-HEAD", "n": n_s, "venue": "loopback"}, f, indent=2)
    f.write("\n")
print(f"G8-local-forge: samples -> {out} ({len(samples)})")
PY

write_receipt "claimed-wire" "loopback smart-HTTP clone/push/fetch ok branch=${BRANCH} tip=${TIP}; not GitHub live PASS"
echo "G8-local-forge: WIRE OK (local peer only; ≠ GitHub live PASS)"
