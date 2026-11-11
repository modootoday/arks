#!/usr/bin/env bash
# Claimed residual gate B7 — zero-downtime hot-rebind.
# SoT: .spec/sot/arks-claimed-residual-gates.sot.md
# research:root/20261111090827 (IW-B7-SSH / IW-B7-FORGE dual-bind drain)
# Scenarios: http (Claimed), ssh (IW-B7-SSH), forge (IW-B7-FORGE).
# Separate receipts — never reuse HTTP Claimed for SSH/forge.
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

alloc_ports() {
  python3 - <<'PY'
import socket
ports = []
for _ in range(2):
    s = socket.socket()
    s.bind(("127.0.0.1", 0))
    ports.append(s.getsockname()[1])
    s.close()
print(*ports)
PY
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

write_receipt() {
  local id="$1" scenario="$2" old="$3" new="$4" note="$5"
  if [[ ! -f /.dockerenv ]]; then
    echo "claimed-rebind-e2e: host run only; skip evidence for $scenario"
    return 0
  fi
  local out=".spec/evidence/${id}"
  mkdir -p "$out"
  python3 - "$out/receipt.json" "$scenario" "$old" "$new" "$note" <<'PY'
import json, sys, time
out, scenario, old, new, note = sys.argv[1:]
receipt = {
    "schema": "arks-claimed-residual/v1",
    "gate": "B7",
    "scenario": scenario,
    "disposition": f"verified-{scenario}-dual-bind",
    "venue": "docker-dod-claimed-rebind",
    "old_listener": old,
    "new_listener": new,
    "in_flight_old_connection": "completed",
    "overlap_fresh_old": "completed",
    "overlap_fresh_new": "completed",
    "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
    "note": note,
}
with open(out, "w", encoding="utf-8") as f:
    json.dump(receipt, f, indent=2)
    f.write("\n")
PY
  echo "claimed-rebind-e2e: receipt=$out/receipt.json"
}

# --- Scenario: HTTP (B7 Claimed baseline) ---
read -r OLD_PORT NEW_PORT < <(alloc_ports)
OLD="127.0.0.1:$OLD_PORT"
NEW="127.0.0.1:$NEW_PORT"
git init --bare -q "$WORK/repo.git"
export ARKS_SERVE_CONFIG="$WORK/serve-http.conf"
printf 'bind=%s\n' "$OLD" >"$ARKS_SERVE_CONFIG"

"$ARKS_BIN" serve-http --listen "$OLD" "$WORK/repo.git" \
  >"$WORK/http.out" 2>"$WORK/http.err" &
SERVER_PID=$!
wait_port "$OLD_PORT"

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
            raise SystemExit("http: new listener not ready after SIGHUP")
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
        raise SystemExit(f"http unexpected response: {data[:120]!r}")

response(held)
response(fresh_old)
response(fresh_new)
PY

grep -F "live rebind dual-bind" "$WORK/http.err" >/dev/null
echo "claimed-rebind-e2e: B7 HTTP checks green (held connection + old/new overlap)"
write_receipt "20261111092000-claimed-rebind-http" "http" "$OLD" "$NEW" \
  "HTTP dual-bind drain; baseline B7 Claimed (≠ SSH/forge)."
kill "$SERVER_PID" 2>/dev/null || true
wait "$SERVER_PID" 2>/dev/null || true
SERVER_PID=""

# --- Scenario: SSH (IW-B7-SSH) ---
command -v ssh-keygen >/dev/null || {
  echo "claimed-rebind-e2e: ssh-keygen missing for SSH scenario" >&2
  exit 1
}
mkdir -p "$WORK/ssh"
ssh-keygen -t ed25519 -N '' -f "$WORK/ssh/host_key" -C arks-b7-host >/dev/null
ssh-keygen -t ed25519 -N '' -f "$WORK/ssh/client" -C arks-b7-client >/dev/null
cat "$WORK/ssh/client.pub" >"$WORK/ssh/authorized_keys"

read -r OLD_PORT NEW_PORT < <(alloc_ports)
OLD="127.0.0.1:$OLD_PORT"
NEW="127.0.0.1:$NEW_PORT"
export HOME="$WORK/home"
mkdir -p "$HOME"
export ARKS_SERVE_CONFIG="$WORK/serve-ssh.conf"
printf 'ssh_listen=%s\n' "$OLD" >"$ARKS_SERVE_CONFIG"

"$ARKS_BIN" serve --no-http --no-forge \
  --ssh-listen "$OLD" \
  --ssh-host-key "$WORK/ssh/host_key" \
  --authorized-keys "$WORK/ssh/authorized_keys" \
  "$WORK/repo.git" >"$WORK/ssh.out" 2>"$WORK/ssh.err" &
SERVER_PID=$!
wait_port "$OLD_PORT"

python3 - "$OLD_PORT" "$NEW_PORT" "$ARKS_SERVE_CONFIG" "$SERVER_PID" <<'PY'
import os, signal, socket, sys, time
old, new, config, pid = int(sys.argv[1]), int(sys.argv[2]), sys.argv[3], int(sys.argv[4])

held = socket.create_connection(("127.0.0.1", old), timeout=2)
# Give accept loop time to spawn the held session thread before rebind.
time.sleep(0.15)
# Do not send peer id yet — hold mid-session while rebind dual-binds.
with open(config, "w", encoding="utf-8") as f:
    f.write(f"ssh_listen=127.0.0.1:{new}\n")
os.kill(pid, signal.SIGHUP)

deadline = time.time() + 10
while True:
    try:
        fresh_new = socket.create_connection(("127.0.0.1", new), timeout=.25)
        break
    except OSError:
        if time.time() >= deadline:
            raise SystemExit("ssh: new listener not ready after SIGHUP")
        time.sleep(.05)

fresh_old = socket.create_connection(("127.0.0.1", old), timeout=2)

def read_banner(sock, label):
    sock.settimeout(5)
    data = b""
    while b"\n" not in data and len(data) < 256:
        part = sock.recv(64)
        if not part:
            break
        data += part
    if not data.startswith(b"SSH-2.0-"):
        raise SystemExit(f"ssh {label} missing banner: {data[:80]!r}")
    return data

# Fresh peers must get banners across dual-bind; held eventually too.
bn_new = read_banner(fresh_new, "fresh_new")
bn_old = read_banner(fresh_old, "fresh_old")
# Complete held peer identification so the session thread finishes cleanly.
held.sendall(b"SSH-2.0-arks_b7_held\r\n")
bn_held = read_banner(held, "held")
for s in (held, fresh_old, fresh_new):
    try:
        s.close()
    except OSError:
        pass
print(f"ssh banners ok held={bn_held[:24]!r} old={bn_old[:24]!r} new={bn_new[:24]!r}")
PY

grep -F "live rebind dual-bind" "$WORK/ssh.err" >/dev/null
echo "claimed-rebind-e2e: B7 SSH checks green (held session + old/new overlap)"
write_receipt "20261111092000-claimed-rebind-ssh" "ssh" "$OLD" "$NEW" \
  "IW-B7-SSH dual-bind drain; separate from HTTP B7 Claimed receipt."
kill "$SERVER_PID" 2>/dev/null || true
wait "$SERVER_PID" 2>/dev/null || true
SERVER_PID=""

# --- Scenario: forge (IW-B7-FORGE) ---
read -r OLD_PORT NEW_PORT < <(alloc_ports)
OLD="127.0.0.1:$OLD_PORT"
NEW="127.0.0.1:$NEW_PORT"
export ARKS_SERVE_CONFIG="$WORK/serve-forge.conf"
printf 'forge_listen=%s\n' "$OLD" >"$ARKS_SERVE_CONFIG"
mkdir -p "$WORK/forge-data"

"$ARKS_BIN" serve --no-http --no-ssh \
  --forge-listen "$OLD" \
  --forge-data "$WORK/forge-data" \
  >"$WORK/forge.out" 2>"$WORK/forge.err" &
SERVER_PID=$!
wait_port "$OLD_PORT"

python3 - "$OLD_PORT" "$NEW_PORT" "$ARKS_SERVE_CONFIG" "$SERVER_PID" <<'PY'
import os, signal, socket, sys, time
old, new, config, pid = int(sys.argv[1]), int(sys.argv[2]), sys.argv[3], int(sys.argv[4])

held = socket.create_connection(("127.0.0.1", old), timeout=2)
held.sendall(b"GET /health HTTP/1.1\r\nHost: held\r\n")
with open(config, "w", encoding="utf-8") as f:
    f.write(f"forge_listen=127.0.0.1:{new}\n")
os.kill(pid, signal.SIGHUP)

deadline = time.time() + 10
while True:
    try:
        fresh_new = socket.create_connection(("127.0.0.1", new), timeout=.25)
        break
    except OSError:
        if time.time() >= deadline:
            raise SystemExit("forge: new listener not ready after SIGHUP")
        time.sleep(.05)

fresh_old = socket.create_connection(("127.0.0.1", old), timeout=2)
request = b"GET /health HTTP/1.1\r\nHost: b7\r\nConnection: close\r\n\r\n"
fresh_old.sendall(request)
fresh_new.sendall(request)
held.sendall(b"Connection: close\r\n\r\n")

def response(sock, label):
    data = b""
    sock.settimeout(5)
    while True:
        try:
            part = sock.recv(65536)
        except socket.timeout:
            break
        if not part:
            break
        data += part
    sock.close()
    if b"HTTP/1.1" not in data[:32]:
        raise SystemExit(f"forge {label} unexpected response: {data[:120]!r}")

response(held, "held")
response(fresh_old, "fresh_old")
response(fresh_new, "fresh_new")
PY

grep -F "live rebind dual-bind" "$WORK/forge.err" >/dev/null
echo "claimed-rebind-e2e: B7 forge checks green (held connection + old/new overlap)"
write_receipt "20261111092000-claimed-rebind-forge" "forge" "$OLD" "$NEW" \
  "IW-B7-FORGE dual-bind drain; ≠ B3 client remote Claimed."
kill "$SERVER_PID" 2>/dev/null || true
wait "$SERVER_PID" 2>/dev/null || true
SERVER_PID=""

echo "claimed-rebind-e2e: PASS — B7 http+ssh+forge dual-bind drain scenarios green"
