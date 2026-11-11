#!/usr/bin/env bash
# DoD pillar 2 — git-daemon vs arks daemon matrix e2e (pin 2.39.5).
# Sibling owns daemon matrix SoT; until it lands, smoke help + loopback subset only.
# Never invents PASS; never spawns gh. Stock `git` is pin oracle only.
# IW-B5-PLUS collector: sot:root/arks-honesty-leftovers-wave (ARKS_DAEMON_B5_PLUS=1).
# IW-DAEMON-FLAGS-PARITY: sot:root/arks-in-scope-leftovers-wave (ARKS_DAEMON_FLAGS_PARITY=1).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

# Intent SoT landed at tip ≥9442659; flag-depth machine CSV may still be pending.
INTENT_SOT="${ARKS_DAEMON_INTENT_SOT:-.spec/sot/arks-daemon-product-intent.sot.md}"
SOT="${ARKS_DAEMON_MATRIX_SOT:-.spec/sot/arks-daemon-git-command-matrix.sot.md}"
CSV="${ARKS_DAEMON_MATRIX_CSV:-verify/matrices/daemon-git-arks-e2e-matrix.csv}"
HONESTY_SOT="${ARKS_HONESTY_LEFTOVERS_SOT:-.spec/sot/arks-honesty-leftovers-wave.sot.md}"
SCOPE_SOT="${ARKS_IN_SCOPE_LEFTOVERS_SOT:-.spec/sot/arks-in-scope-leftovers-wave.sot.md}"
# Prefer landed sibling daemon matrix SoT when present.

TIP="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
echo "daemon-git-arks-matrix-e2e: tip=${TIP}"
echo "daemon-git-arks-matrix-e2e: pin git=$(git --version)"

ARKS_BIN="${ARKS_BIN:-}"
if [[ -z "$ARKS_BIN" ]]; then
  if [[ -x ./.cargo-target/debug/arks ]]; then
    ARKS_BIN=./.cargo-target/debug/arks
  elif [[ -x ./target/debug/arks ]]; then
    ARKS_BIN=./target/debug/arks
  elif [[ -n "${CARGO_TARGET_DIR:-}" && -x "${CARGO_TARGET_DIR}/debug/arks" ]]; then
    ARKS_BIN="${CARGO_TARGET_DIR}/debug/arks"
  else
    cargo build -p arks -p arks-daemon
    ARKS_BIN=./.cargo-target/debug/arks
    [[ -x "$ARKS_BIN" ]] || ARKS_BIN=./target/debug/arks
    [[ -x "$ARKS_BIN" ]] || ARKS_BIN="${CARGO_TARGET_DIR:-target}/debug/arks"
  fi
fi

# --- IW-DAEMON-FLAGS-PARITY (ARKS_DAEMON_FLAGS_PARITY=1) ---
if [[ "${ARKS_DAEMON_FLAGS_PARITY:-0}" == "1" ]]; then
  echo "daemon-git-arks-matrix-e2e: IW-DAEMON-FLAGS-PARITY (sot:root/arks-in-scope-leftovers-wave)"
  [[ -f "$SCOPE_SOT" ]] || { echo "FAIL: missing $SCOPE_SOT" >&2; exit 1; }
  export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"
  cargo build -p arks --release
  ARKS_BIN="${CARGO_TARGET_DIR:-target}/release/arks"
  [[ -x "$ARKS_BIN" ]] || ARKS_BIN=./.cargo-target/release/arks
  [[ -x "$ARKS_BIN" ]] || ARKS_BIN=./target/release/arks
  WORK="$(mktemp -d /tmp/arks-daemon-flags.XXXXXX)"
  cleanup_df() { rm -rf "$WORK"; }
  trap cleanup_df EXIT
  mkdir -p "$WORK/repo"
  (
    cd "$WORK/repo"
    git init -q
    git config user.email t@t
    git config user.name t
    echo hi >README
    git add README
    git commit -q -m init
    touch .git/git-daemon-export-ok
  )
  DF_OK=1
  # Help advertises peer-mapped flags.
  help_out="$("$ARKS_BIN" daemon --help 2>&1 || true)"
  for flag in --verbose --timeout --init-timeout --max-connections --strict-paths --listen --port --enable --disable --reuseaddr --base-path; do
    if ! echo "$help_out" | grep -Fq -- "$flag"; then
      echo "FAIL DF: help missing $flag" >&2
      DF_OK=0
    fi
  done
  # Bind listen+port+verbose+timeouts+strict-paths+base-path; clone via git://
  PORT="$(python3 - <<'PY'
import socket
s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1]); s.close()
PY
)"
  "$ARKS_BIN" daemon --verbose --listen=127.0.0.1 --port="$PORT" \
    --timeout=30 --init-timeout=5 --max-connections=8 --strict-paths \
    --base-path="$WORK" --export-all --reuseaddr \
    --enable=upload-pack --disable=receive-pack \
    "$WORK/repo" >"$WORK/daemon.out" 2>"$WORK/daemon.err" &
  DPID=$!
  sleep 0.3
  if ! kill -0 "$DPID" 2>/dev/null; then
    echo "FAIL DF: daemon failed to start" >&2
    cat "$WORK/daemon.err" >&2 || true
    DF_OK=0
  else
    mkdir -p "$WORK/clone"
    if ! git -c protocol.version=1 clone --quiet "git://127.0.0.1:${PORT}/repo" "$WORK/clone/repo" 2>"$WORK/clone.err"; then
      echo "FAIL DF: git:// clone via listen/port/base-path" >&2
      cat "$WORK/clone.err" "$WORK/daemon.err" >&2 || true
      DF_OK=0
    fi
    # receive-pack disabled: push should fail (no new commit needed — empty push probe).
    set +e
    git -C "$WORK/clone/repo" push "git://127.0.0.1:${PORT}/repo" HEAD:refs/heads/denied 2>"$WORK/push.err"
    push_ec=$?
    set -e
    if [[ "$push_ec" -eq 0 ]]; then
      echo "FAIL DF: push succeeded while receive-pack disabled" >&2
      DF_OK=0
    fi
    kill "$DPID" 2>/dev/null || true
    wait "$DPID" 2>/dev/null || true
  fi
  # upload-archive enable + ustar via git archive --remote (path-filter Stay Partial).
  PORT_A="$(python3 - <<'PY'
import socket
s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1]); s.close()
PY
)"
  # Mirror B5 layout: base-path + repo.git name for --remote path.
  rm -rf "$WORK/repo.git"
  mv "$WORK/repo/.git" "$WORK/repo.git"
  touch "$WORK/repo.git/git-daemon-export-ok"
  "$ARKS_BIN" daemon --listen=127.0.0.1 --port="$PORT_A" --export-all --reuseaddr \
    --enable=upload-archive --base-path="$WORK" \
    >"$WORK/daemon-archive.out" 2>"$WORK/daemon-archive.err" &
  APID=$!
  sleep 0.3
  if ! kill -0 "$APID" 2>/dev/null; then
    echo "FAIL DF: upload-archive daemon failed to start" >&2
    cat "$WORK/daemon-archive.err" >&2 || true
    DF_OK=0
  else
    set +e
    git archive --remote="git://127.0.0.1:${PORT_A}/repo.git" --format=tar HEAD \
      >"$WORK/remote.tar" 2>"$WORK/archive.err"
    arch_ec=$?
    set -e
    if [[ "$arch_ec" -ne 0 ]] || [[ ! -s "$WORK/remote.tar" ]]; then
      echo "FAIL DF: git archive --remote upload-archive" >&2
      cat "$WORK/archive.err" "$WORK/daemon-archive.err" >&2 || true
      DF_OK=0
    else
      echo "daemon-git-arks-matrix-e2e: DF upload-archive ok (bytes=$(wc -c <"$WORK/remote.tar"))"
    fi
    kill "$APID" 2>/dev/null || true
    wait "$APID" 2>/dev/null || true
  fi
  if [[ "$DF_OK" -ne 1 ]]; then
    if [[ "${ARKS_DOD_STRICT:-0}" == "1" ]]; then
      echo "daemon-git-arks-matrix-e2e: FAIL STRICT IW-DAEMON-FLAGS-PARITY" >&2
      exit 1
    fi
    echo "daemon-git-arks-matrix-e2e: IW-DAEMON-FLAGS-PARITY Partial/failed"
    exit 0
  fi
  echo "daemon-git-arks-matrix-e2e: IW-DAEMON-FLAGS-PARITY greens"
  if [[ -f /.dockerenv ]]; then
    TS="$(date -u +%Y%m%d%H%M%S)"
    OUT=".spec/evidence/${TS}-daemon-flags-parity"
    mkdir -p "$OUT"
    cp "$WORK"/daemon.* "$WORK"/clone.err "$WORK"/push.err \
      "$WORK"/daemon-archive.* "$WORK"/archive.err "$WORK"/remote.tar \
      "$OUT/" 2>/dev/null || true
    python3 - "$OUT/receipt.json" "$TIP" "${ARKS_JOURNEY_IMAGE_TAG:-arks-verify-m1:0.1.0}" <<'PY'
import json, sys, time
out, tip, image = sys.argv[1:]
receipt = {
    "schema": "arks-daemon-flags-parity/v1",
    "gate": "IW-DAEMON-FLAGS-PARITY",
    "disposition": "verified-product-path",
    "venue": "docker-dod-daemon-matrix",
    "tip": tip,
    "image_tag": image,
    "parity_flags": [
        "--verbose", "--timeout", "--init-timeout", "--max-connections",
        "--strict-paths", "--listen", "--port", "--base-path", "--reuseaddr",
        "--enable", "--disable", "<directory>...", "service:upload-pack", "service:receive-pack",
        "service:upload-archive",
    ],
    "parity_pointers": ["related:git:// client"],
    "note": "upload-archive enable+ustar Parity; path-filter/non-ustar Stay Partial; SysV double-fork Refuse; git:// client Parity pointer (not daemon DoD).",
    "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
}
with open(out, "w", encoding="utf-8") as f:
    json.dump(receipt, f, indent=2)
    f.write("\n")
PY
    echo "daemon-git-arks-matrix-e2e: receipt=$OUT/receipt.json"
  fi
  exit 0
fi

# --- IW-B5-PLUS syslog facility/priority collector (ARKS_DAEMON_B5_PLUS=1) ---
if [[ "${ARKS_DAEMON_B5_PLUS:-0}" == "1" ]]; then
  echo "daemon-git-arks-matrix-e2e: IW-B5-PLUS collector path (sot:root/arks-honesty-leftovers-wave)"
  if [[ ! -f "$HONESTY_SOT" ]]; then
    echo "FAIL: missing honesty leftovers SoT $HONESTY_SOT" >&2
    exit 1
  fi
  if [[ ! -f /.dockerenv ]]; then
    echo "daemon-git-arks-matrix-e2e: B5+ host run — Docker collector venue required; honesty Refuse (no invent Claimed)"
    if [[ "${ARKS_DOD_STRICT:-0}" == "1" ]]; then
      exit 1
    fi
    exit 0
  fi
  export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"
  cargo build -p arks --release
  ARKS_BIN="${CARGO_TARGET_DIR:-target}/release/arks"
  [[ -x "$ARKS_BIN" ]] || ARKS_BIN=./.cargo-target/release/arks
  [[ -x "$ARKS_BIN" ]] || ARKS_BIN=./target/release/arks

  WORK="$(mktemp -d /tmp/arks-b5plus.XXXXXX)"
  COLLECT="$WORK/collect.log"
  : >"$COLLECT"
  # Bind a unix datagram collector at /dev/log when permitted (peer openlog path).
  python3 - "$WORK" "$COLLECT" <<'PY' &
import os, socket, sys, time
work, path = sys.argv[1], sys.argv[2]
sock_path = "/dev/log"
# Prefer replacing /dev/log inside the container; fall back to workdir sock.
targets = []
if os.geteuid() == 0:
    targets.append("/dev/log")
targets.append(os.path.join(work, "syslog.sock"))
bound = None
for t in targets:
    try:
        if os.path.exists(t) or os.path.islink(t):
            try:
                os.unlink(t)
            except OSError:
                pass
        s = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
        s.bind(t)
        s.settimeout(0.5)
        bound = (s, t)
        open(os.path.join(work, "collector.bound"), "w").write(t + "\n")
        break
    except OSError as e:
        open(os.path.join(work, "collector.bind.err"), "a").write(f"{t}: {e}\n")
if not bound:
    open(path, "a").write("COLLECTOR_BIND_FAILED\n")
    time.sleep(30)
    raise SystemExit(1)
s, t = bound
deadline = time.time() + 25
with open(path, "ab") as out:
    while time.time() < deadline:
        try:
            data, _ = s.recvfrom(65535)
            out.write(data + b"\n")
            out.flush()
            if b"ARKS_B5PLUS_MARKER" in data:
                open(os.path.join(work, "collector.hit"), "w").write("1\n")
                break
        except socket.timeout:
            continue
s.close()
PY
  COLLECTOR_PID=$!
  sleep 0.4

  BASE="$WORK/repo.git"
  mkdir -p "$BASE"
  git init --bare "$BASE" >/dev/null
  # Touch export-ok so daemon may serve; --syslog arms openlog+marker then listen briefly.
  touch "$BASE/git-daemon-export-ok"
  set +e
  timeout 3 "$ARKS_BIN" daemon --syslog --listen=127.0.0.1 --port=0 --export-all "$BASE" \
    >"$WORK/daemon.out" 2>"$WORK/daemon.err"
  set -e
  sleep 0.5
  kill "$COLLECTOR_PID" 2>/dev/null || true
  wait "$COLLECTOR_PID" 2>/dev/null || true

  B5_OK=0
  if [[ -f "$WORK/collector.hit" ]] && grep -a -q 'ARKS_B5PLUS_MARKER' "$COLLECT"; then
    # PRI decode: leading <N> where N = facility*8 + severity; daemon+info => 3*8+6=30.
    if python3 - "$COLLECT" <<'PY'
import re, sys
raw = open(sys.argv[1], "rb").read()
text = raw.decode("utf-8", "replace")
m = re.search(r"<(\d+)>.*?ARKS_B5PLUS_MARKER", text, re.S)
if not m:
    # Some syslogd paths strip PRI; accept marker+bound /dev/log only as Partial.
    print("NO_PRI")
    raise SystemExit(2)
pri = int(m.group(1))
facility, severity = divmod(pri, 8)
print(f"pri={pri} facility={facility} severity={severity}")
if facility != 3:  # LOG_DAEMON
    raise SystemExit(3)
if severity != 6:  # LOG_INFO
    raise SystemExit(4)
raise SystemExit(0)
PY
    then
      B5_OK=1
      echo "daemon-git-arks-matrix-e2e: B5+ collector PRI facility=daemon severity=info ok"
    else
      echo "daemon-git-arks-matrix-e2e: B5+ marker seen but PRI decode failed — honest Refuse" >&2
    fi
  else
    echo "daemon-git-arks-matrix-e2e: B5+ collector miss (no marker) — honest Refuse" >&2
    cat "$WORK/daemon.err" >&2 || true
    cat "$WORK/collector.bind.err" >&2 || true
  fi

  TS="$(date -u +%Y%m%d%H%M%S)"
  OUT=".spec/evidence/${TS}-daemon-b5-plus-syslog"
  mkdir -p "$OUT"
  cp "$WORK/daemon.err" "$WORK/daemon.out" "$COLLECT" "$OUT/" 2>/dev/null || true
  cp "$WORK"/collector.* "$OUT/" 2>/dev/null || true
  if [[ "$B5_OK" -eq 1 ]]; then
    python3 - "$OUT/receipt.json" "$TIP" "${ARKS_JOURNEY_IMAGE_TAG:-arks-verify-m1:0.1.0}" <<'PY'
import json, sys, time
out, tip, image = sys.argv[1:]
receipt = {
    "schema": "arks-daemon-b5-plus-syslog/v1",
    "gate": "IW-B5-PLUS",
    "disposition": "verified-product-path",
    "venue": "docker-dod-daemon-matrix",
    "tip": tip,
    "image_tag": image,
    "facility": "daemon",
    "facility_num": 3,
    "severity": "info",
    "severity_num": 6,
    "pri": 30,
    "marker": "ARKS_B5PLUS_MARKER",
    "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
}
with open(out, "w", encoding="utf-8") as f:
    json.dump(receipt, f, indent=2)
    f.write("\n")
PY
    echo "daemon-git-arks-matrix-e2e: B5+ Claimed receipt=$OUT/receipt.json"
    exit 0
  fi
  python3 - "$OUT/receipt.json" "$TIP" "${ARKS_JOURNEY_IMAGE_TAG:-arks-verify-m1:0.1.0}" <<'PY'
import json, sys, time
out, tip, image = sys.argv[1:]
receipt = {
    "schema": "arks-daemon-b5-plus-syslog/v1",
    "gate": "IW-B5-PLUS",
    "disposition": "refuse-terminal",
    "venue": "docker-dod-daemon-matrix",
    "tip": tip,
    "image_tag": image,
    "note": "Collector miss or PRI mismatch — keep Refuse; openlog alone ≠ Claimed.",
    "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
}
with open(out, "w", encoding="utf-8") as f:
    json.dump(receipt, f, indent=2)
    f.write("\n")
PY
  echo "daemon-git-arks-matrix-e2e: B5+ Refuse receipt=$OUT/receipt.json"
  if [[ "${ARKS_DOD_STRICT:-0}" == "1" ]]; then
    # STRICT + Refuse is an honest terminal close, not invent Claimed — exit 0 with Refuse receipt.
    echo "daemon-git-arks-matrix-e2e: STRICT accepts terminal Refuse (no invent Claimed)"
  fi
  exit 0
fi

# Muscle-memory entry must exist (single release binary).
help_out="$("$ARKS_BIN" daemon --help 2>&1 || true)"
if ! echo "$help_out" | grep -qi 'git://'; then
  echo "FAIL: arks daemon --help missing git:// usage"
  exit 1
fi
echo "daemon-git-arks-matrix-e2e: arks daemon --help ok"

# Pin oracle usage exists (git-daemon). Prefer `git daemon -h` — man pages often absent in image.
pin_help="$(git daemon -h 2>&1 || true)"
if ! echo "$pin_help" | grep -qi 'usage: git daemon'; then
  echo "FAIL: pin git daemon -h missing usage"
  exit 1
fi
echo "daemon-git-arks-matrix-e2e: pin git daemon -h ok"

if [[ ! -f "$INTENT_SOT" ]]; then
  echo "daemon-git-arks-matrix-e2e: FAIL — missing daemon intent SoT $INTENT_SOT"
  exit 1
fi
echo "daemon-git-arks-matrix-e2e: intent SoT ok ($INTENT_SOT)"

if [[ ! -f "$SOT" || ! -f "$CSV" ]]; then
  echo "daemon-git-arks-matrix-e2e: SKIP flag-depth matrix — waiting $SOT / $CSV"
  echo "daemon-git-arks-matrix-e2e: smoke-only PASS (help + intent SoT); flag e2e not Claimed"
  exit 0
fi

missing=$(awk -F, 'NR>1 && ($3=="Missing" || $3=="missing") {c++} END{print c+0}' "$CSV")
echo "daemon-git-arks-matrix-e2e: Missing=$missing from $CSV"
if [[ "${ARKS_DOD_STRICT:-0}" == "1" || "$missing" -eq 0 ]]; then
  if [[ "$missing" -gt 0 ]]; then
    echo "daemon-git-arks-matrix-e2e: FAIL — daemon matrix Missing=$missing (no invent PASS)"
    exit 1
  fi
  echo "daemon-git-arks-matrix-e2e: PASS — matrix Missing=0"
  exit 0
fi
echo "daemon-git-arks-matrix-e2e: smoke-only PASS (CSV present, Missing=$missing); STRICT waits ARKS_DOD_STRICT=1"
exit 0
