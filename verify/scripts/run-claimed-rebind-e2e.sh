#!/usr/bin/env bash
# Claimed residual gate B7 — zero-downtime hot-rebind.
# SoT: .spec/sot/arks-claimed-residual-gates.sot.md
# Scenario matrix: HTTP smart-serve is exercised here. SSH crypto is deliberately
# out of scope; forge can use the same dual-bind pattern in a later scenario.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

SOT="${ARKS_CLAIMED_RESIDUAL_SOT:-.spec/sot/arks-claimed-residual-gates.sot.md}"
if [[ ! -f "$SOT" ]]; then
  echo "claimed-rebind-e2e: missing SoT $SOT" >&2
  exit 1
fi

command -v python3 >/dev/null || {
  echo "claimed-rebind-e2e: python3 missing" >&2
  exit 1
}
command -v git >/dev/null || {
  echo "claimed-rebind-e2e: stock git fixture setup missing" >&2
  exit 1
}

export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"
cargo build -p arks --release
ARKS_BIN="${ARKS_BIN:-${CARGO_TARGET_DIR:-target}/release/arks}"
[[ -x "$ARKS_BIN" ]] || {
  echo "claimed-rebind-e2e: arks binary missing: $ARKS_BIN" >&2
  exit 1
}

WORK="$(mktemp -d /tmp/arks-b7.XXXXXX)"
SERVER_PID=""
cleanup() {
  if [[ -n "$SERVER_PID" ]] && kill -0 "$SERVER_PID" 2>/dev/null; then
    kill "$SERVER_PID" 2>/dev/null || true
    wait "$SERVER_PID" 2>/dev/null || true
  fi
  rm -rf "$WORK"
}
trap cleanup EXIT

read -r OLD_PORT NEW_PORT < <(python3 - <<'PY'
import socket
ports = []
for _ in range(2):
    s = socket.socket()
    s.bind(("127.0.0.1", 0))
    ports.append(s.getsockname()[1])
    s.close()
print(*ports)
PY
)
OLD="127.0.0.1:$OLD_PORT"
NEW="127.0.0.1:$NEW_PORT"
git init --bare -q "$WORK/repo.git"
export ARKS_SERVE_CONFIG="$WORK/serve.conf"
printf 'bind=%s\n' "$OLD" >"$ARKS_SERVE_CONFIG"

"$ARKS_BIN" serve-http --listen "$OLD" "$WORK/repo.git" \
  >"$WORK/server.out" 2>"$WORK/server.err" &
SERVER_PID=$!
python3 - "$OLD_PORT" <<'PY'
import socket, sys, time
port = int(sys.argv[1])
for _ in range(100):
    try:
        with socket.create_connection(("127.0.0.1", port), timeout=.2):
            raise SystemExit(0)
    except OSError:
        time.sleep(.05)
raise SystemExit("old listener not ready")
PY

# Hold one request open before rebind. It must finish after rebind, while fresh
# peers also succeed through both old (draining) and new listeners.
python3 - "$OLD_PORT" "$NEW_PORT" "$ARKS_SERVE_CONFIG" "$SERVER_PID" <<'PY'
import os, signal, socket, sys, time
old, new, config, pid = int(sys.argv[1]), int(sys.argv[2]), sys.argv[3], int(sys.argv[4])

held = socket.create_connection(("127.0.0.1", old), timeout=2)
held.sendall(b"GET /info/refs HTTP/1.1\r\nHost: held\r\n")
with open(config, "w", encoding="utf-8") as f:
    f.write(f"bind=127.0.0.1:{new}\n")
os.kill(pid, signal.SIGHUP)

deadline = time.time() + 10
while True:
    try:
        fresh_new = socket.create_connection(("127.0.0.1", new), timeout=.25)
        break
    except OSError:
        if time.time() >= deadline:
            raise SystemExit("new listener not ready after SIGHUP")
        time.sleep(.05)

fresh_old = socket.create_connection(("127.0.0.1", old), timeout=2)
request = b"GET /info/refs HTTP/1.1\r\nHost: b7\r\nConnection: close\r\n\r\n"
fresh_old.sendall(request)
fresh_new.sendall(request)
held.sendall(b"Connection: close\r\n\r\n")

def response(sock):
    data = b""
    while True:
        part = sock.recv(65536)
        if not part:
            break
        data += part
    sock.close()
    if not data.startswith(b"HTTP/1.1 200 "):
        raise SystemExit(f"unexpected HTTP response: {data[:120]!r}")

response(held)
response(fresh_old)
response(fresh_new)
PY

grep -F "live rebind dual-bind" "$WORK/server.err" >/dev/null
echo "claimed-rebind-e2e: B7 HTTP checks green (held connection + old/new overlap)"

if [[ -f /.dockerenv ]]; then
  TS="$(date -u +%Y%m%d%H%M%S)"
  OUT=".spec/evidence/${TS}-claimed-rebind"
  mkdir -p "$OUT"
  python3 - "$OUT/receipt.json" "$OLD" "$NEW" <<'PY'
import json, sys, time
out, old, new = sys.argv[1:]
receipt = {
    "schema": "arks-claimed-residual/v1",
    "gate": "B7",
    "disposition": "verified-http-dual-bind",
    "venue": "docker-loopback",
    "old_listener": old,
    "new_listener": new,
    "in_flight_old_connection": "completed",
    "overlap_fresh_old": "completed",
    "overlap_fresh_new": "completed",
    "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
    "note": "HTTP scenario only; no SSH crypto claim.",
}
with open(out, "w", encoding="utf-8") as f:
    json.dump(receipt, f, indent=2)
    f.write("\n")
PY
  echo "claimed-rebind-e2e: receipt=$OUT/receipt.json"
else
  echo "claimed-rebind-e2e: host run only; no .spec/evidence receipt written"
fi
