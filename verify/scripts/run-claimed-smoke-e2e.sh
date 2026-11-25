#!/usr/bin/env bash
# Queue unit Q21 — serve / agent smoke diagnostics.
# SoT: .spec/sot/arks-claimed-residual-gates.sot.md §Q21
# Claims the smoke commands as diagnostics only (correct report + failure exit);
# a smoke green never stands in for B1/B2/B4. Product MUST NOT spawn ssh/git/gh/ollama.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

SOT="${ARKS_CLAIMED_RESIDUAL_SOT:-.spec/sot/arks-claimed-residual-gates.sot.md}"
TIP="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
echo "claimed-smoke-e2e: tip=${TIP}"
[[ -f "$SOT" ]] || {
  echo "claimed-smoke-e2e: missing SoT $SOT" >&2
  exit 1
}
command -v python3 >/dev/null

export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"
cargo build -p arks --release
ARKS_BIN="${ARKS_BIN:-${CARGO_TARGET_DIR:-target}/release/arks}"
[[ -x "$ARKS_BIN" ]] || {
  echo "claimed-smoke-e2e: arks binary missing: $ARKS_BIN" >&2
  exit 1
}

WORK="$(mktemp -d /tmp/arks-q21.XXXXXX)"
PIDS=()
cleanup() {
  for p in "${PIDS[@]}"; do
    kill "$p" 2>/dev/null || true
    wait "$p" 2>/dev/null || true
  done
  rm -rf "$WORK"
}
trap cleanup EXIT
export HOME="$WORK/home"
export ARKS_SERVE_CONFIG="$WORK/serve.conf"
mkdir -p "$HOME"

free_port() {
  python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1])'
}
wait_port() {
  python3 - "$1" <<'PY'
import socket, sys, time
port = int(sys.argv[1])
for _ in range(200):
    try:
        with socket.create_connection(("127.0.0.1", port), timeout=.2):
            raise SystemExit(0)
    except OSError:
        time.sleep(.05)
raise SystemExit(f"listener {port} not ready")
PY
}
# expect_rc NAME RC -- command...  (stdout+stderr kept under $WORK/NAME.*)
expect_rc() {
  local name="$1" want="$2"
  shift 3
  set +e
  "$@" >"$WORK/$name.out" 2>"$WORK/$name.err"
  local rc=$?
  set -e
  if [[ "$rc" -ne "$want" ]]; then
    echo "claimed-smoke-e2e: $name rc=$rc want=$want" >&2
    cat "$WORK/$name.out" "$WORK/$name.err" >&2
    exit 1
  fi
  echo "claimed-smoke-e2e: $name rc=$rc"
}
has() {
  grep -Fx "$2" "$WORK/$1.out" >/dev/null || {
    echo "claimed-smoke-e2e: $1 lacks line: $2" >&2
    cat "$WORK/$1.out" >&2
    exit 1
  }
}

# --- arks serve ssh-smoke against a durable `arks serve` SSH listener ---
"$ARKS_BIN" init -q --bare "$WORK/repo.git"
SSH_PORT="$(free_port)"
"$ARKS_BIN" serve --no-http --no-forge --ssh-listen "127.0.0.1:$SSH_PORT" "$WORK/repo.git" \
  >"$WORK/serve.out" 2>"$WORK/serve.err" &
PIDS+=("$!")
wait_port "$SSH_PORT"
expect_rc ssh-banner 0 -- "$ARKS_BIN" serve ssh-smoke --listen "127.0.0.1:$SSH_PORT"
has ssh-banner "ssh_banner=ok"
has ssh-banner "banner_preview=SSH-2.0-arks"
expect_rc ssh-git-pack 0 -- "$ARKS_BIN" serve ssh-smoke --git-pack
has ssh-git-pack "server_kexinit=ok"
has ssh-git-pack "git_over_ssh_smoke=ok"
CLOSED="$(free_port)"
expect_rc ssh-closed 1 -- "$ARKS_BIN" serve ssh-smoke --listen "127.0.0.1:$CLOSED"
has ssh-closed "ssh_banner=none"

# --- arks agent provider smoke / review --smoke against the openai-compat fixture ---
LLM_PORT="$(free_port)"
BASE_URL="http://127.0.0.1:${LLM_PORT}/v1"
python3 verify/fixtures/openai-compat-fixture.py --port "$LLM_PORT" \
  --log "$WORK/requests.jsonl" >"$WORK/fixture.log" 2>&1 &
PIDS+=("$!")
wait_port "$LLM_PORT"
expect_rc provider-fixture 0 -- "$ARKS_BIN" agent provider smoke openai-compat \
  --base-url "$BASE_URL" --model arks-fixture
has provider-fixture "chat_http=200"
has provider-fixture "smoke_result=ok"
expect_rc provider-mock 0 -- "$ARKS_BIN" agent provider smoke --mock
has provider-mock "smoke_result=ok"
expect_rc provider-closed 1 -- "$ARKS_BIN" agent provider smoke openai-compat \
  --base-url "http://127.0.0.1:${CLOSED}/v1"
has provider-closed "smoke_result=fail"
expect_rc review-fixture 0 -- "$ARKS_BIN" agent review --smoke \
  --base-url "$BASE_URL" --model arks-fixture
has review-fixture "review_smoke=ok"
expect_rc review-mock 0 -- "$ARKS_BIN" agent review --smoke --mock
has review-mock "review_smoke=ok"
expect_rc review-closed 1 -- "$ARKS_BIN" agent review --smoke \
  --base-url "http://127.0.0.1:${CLOSED}/v1"
has review-closed "review_smoke=fail"
FIXTURE_REQUESTS="$(wc -l <"$WORK/requests.jsonl")"
[[ "$FIXTURE_REQUESTS" -eq 2 ]] || {
  echo "claimed-smoke-e2e: fixture saw $FIXTURE_REQUESTS chat requests, want 2" >&2
  exit 1
}
echo "claimed-smoke-e2e: all smoke checks green"

if [[ ! -f /.dockerenv ]]; then
  echo "claimed-smoke-e2e: host run only; no evidence written"
  exit 0
fi
TS="$(date -u +%Y%m%d%H%M%S)"
OUT="${ARKS_EVIDENCE_ROOT:-.spec/evidence}/${TS}-claimed-smoke"
mkdir -p "$OUT/logs"
# Public evidence: temp roots end in a home directory and read as local user paths to the
# public-surface scan, so tracked copies write them <tmp-home>.
for f in "$WORK"/*.out "$WORK"/*.err; do sed -E 's#/tmp/[^ /]+/home#<tmp-home>#g' "$f" >"$OUT/logs/${f##*/}"; done
python3 - "$OUT/receipt.json" "$TIP" "${ARKS_JOURNEY_IMAGE_TAG:-arks-verify-m1:0.1.0}" <<'PY'
import json, sys, time
out, tip, image = sys.argv[1:]
receipt = {
    "schema": "arks-claimed-residual/v1",
    "gate": "Q21-smokes",
    "disposition": "verified-smoke-diagnostics",
    "venue": "docker-dod-claimed-smoke",
    "tip": tip,
    "image_tag": image,
    "checks": [
        "serve ssh-smoke --listen: ssh_banner=ok rc=0 against arks serve SSH",
        "serve ssh-smoke --git-pack: server_kexinit=ok rc=0",
        "serve ssh-smoke closed port: rc=1",
        "agent provider smoke fixture over http: chat_http=200 rc=0",
        "agent provider smoke --mock: rc=0",
        "agent provider smoke closed port: rc=1",
        "agent review --smoke fixture: review_smoke=ok rc=0",
        "agent review --smoke --mock: rc=0",
        "agent review --smoke closed port: rc=1",
        "fixture logged exactly 2 chat requests",
    ],
    "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
    "note": "Diagnostics only; does not prove B1/B2 or B4. HTTPS vendor endpoints not exercised.",
}
with open(out, "w", encoding="utf-8") as f:
    json.dump(receipt, f, indent=2)
    f.write("\n")
PY
echo "claimed-smoke-e2e: receipt=$OUT/receipt.json"
