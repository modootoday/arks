#!/usr/bin/env bash
# Claimed residual gate B4 — agent LLM chatbot/review e2e.
# SoT: .spec/sot/arks-claimed-residual-gates.sot.md
# Runs a real two-turn product chat plus a separate review request against the
# deterministic OpenAI-compatible fixture. Product MUST NOT spawn ollama/git/gh.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

SOT="${ARKS_CLAIMED_RESIDUAL_SOT:-.spec/sot/arks-claimed-residual-gates.sot.md}"
if [[ ! -f "$SOT" ]]; then
  echo "claimed-agent-llm-e2e: missing SoT $SOT" >&2
  exit 1
fi

command -v python3 >/dev/null || {
  echo "claimed-agent-llm-e2e: python3 fixture runtime missing" >&2
  exit 1
}

export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"
cargo build -p arks --release
ARKS_BIN="${ARKS_BIN:-${CARGO_TARGET_DIR:-target}/release/arks}"
if [[ ! -x "$ARKS_BIN" ]]; then
  echo "claimed-agent-llm-e2e: arks binary missing: $ARKS_BIN" >&2
  exit 1
fi

WORK="$(mktemp -d /tmp/arks-b4.XXXXXX)"
FIXTURE_PID=""
cleanup() {
  if [[ -n "$FIXTURE_PID" ]] && kill -0 "$FIXTURE_PID" 2>/dev/null; then
    kill "$FIXTURE_PID" 2>/dev/null || true
    wait "$FIXTURE_PID" 2>/dev/null || true
  fi
  rm -rf "$WORK"
}
trap cleanup EXIT

PORT="$(python3 - <<'PY'
import socket
s = socket.socket()
s.bind(("127.0.0.1", 0))
print(s.getsockname()[1])
s.close()
PY
)"
BASE_URL="http://127.0.0.1:${PORT}/v1"
python3 verify/fixtures/openai-compat-fixture.py \
  --port "$PORT" --log "$WORK/requests.jsonl" >"$WORK/fixture.log" 2>&1 &
FIXTURE_PID=$!
python3 - "$PORT" <<'PY'
import socket, sys, time
port = int(sys.argv[1])
for _ in range(100):
    try:
        with socket.create_connection(("127.0.0.1", port), timeout=.2):
            raise SystemExit(0)
    except OSError:
        time.sleep(.05)
raise SystemExit("B4 fixture did not become ready")
PY

"$ARKS_BIN" agent chat \
  --base-url "$BASE_URL" --model arks-fixture \
  --message "first B4 turn" --message "second B4 turn" \
  >"$WORK/chat.out" 2>"$WORK/chat.err"
grep -Fx "assistant_1=fixture-turn-1" "$WORK/chat.out" >/dev/null
grep -Fx "assistant_2=fixture-turn-2-history-ok" "$WORK/chat.out" >/dev/null
grep -Fx "chat_turns_completed=2" "$WORK/chat.out" >/dev/null

printf '%s\n' "B4_REVIEW_TOKEN review this change without inventing status" |
  "$ARKS_BIN" agent review --base-url "$BASE_URL" --model arks-fixture \
  >"$WORK/review.out" 2>"$WORK/review.err"
grep -F "review-result: no invented Claimed or PASS; fixture token observed" \
  "$WORK/review.out" >/dev/null

python3 - "$WORK/requests.jsonl" <<'PY'
import json, sys
rows = [json.loads(line) for line in open(sys.argv[1], encoding="utf-8")]
assert len(rows) == 3, f"expected 3 fixture requests, got {len(rows)}"
second = rows[1]["messages"]
assert [m["role"] for m in second] == ["user", "assistant", "user"], second
assert second[1]["content"] == "fixture-turn-1", second
assert "B4_REVIEW_TOKEN" in rows[2]["messages"][0]["content"], rows[2]
PY

echo "claimed-agent-llm-e2e: B4 product checks green (two-turn history + review)"

# Evidence is emitted only by the authoritative Docker venue after all checks.
if [[ -f /.dockerenv ]]; then
  TS="$(date -u +%Y%m%d%H%M%S)"
  OUT=".spec/evidence/${TS}-claimed-agent-llm"
  mkdir -p "$OUT"
  python3 - "$OUT/receipt.json" "${ARKS_JOURNEY_IMAGE_TAG:-arks-verify-m1:0.1.0}" <<'PY'
import json, sys, time
out, image = sys.argv[1:]
receipt = {
    "schema": "arks-claimed-residual/v1",
    "gate": "B4",
    "disposition": "verified-product-path",
    "venue": "docker-openai-compat-fixture",
    "image_tag": image,
    "chat_turns": 2,
    "review_requests": 1,
    "checks": ["assistant history retained", "review used native HTTP"],
    "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
    "note": "Receipt records observed product behavior; it does not by itself amend SoT status.",
}
with open(out, "w", encoding="utf-8") as f:
    json.dump(receipt, f, indent=2)
    f.write("\n")
PY
  echo "claimed-agent-llm-e2e: receipt=$OUT/receipt.json"
else
  echo "claimed-agent-llm-e2e: host run only; no .spec/evidence receipt written"
fi
