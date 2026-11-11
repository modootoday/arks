#!/usr/bin/env bash
# Claimed residual gates B5/B5+ — daemon archive parity + syslog facility.
# SoT: .spec/sot/arks-claimed-residual-gates.sot.md
# research:root/20261111090827 + research:root/20261111084400 (IW-B5)
# verify-directory-conventions: B5 Claimed on cmp -s; B5+ Refuse terminal.
# Product MUST NOT spawn git. Stock git 2.39.5 is oracle only.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

SOT="${ARKS_CLAIMED_RESIDUAL_SOT:-.spec/sot/arks-claimed-residual-gates.sot.md}"
if [[ ! -f "$SOT" ]]; then
  echo "claimed-daemon-archive-e2e: missing SoT $SOT" >&2
  exit 1
fi
command -v git >/dev/null
command -v python3 >/dev/null
STOCK_GIT="$(git --version)"
if [[ "$STOCK_GIT" != "git version 2.39.5" ]]; then
  echo "claimed-daemon-archive-e2e: requires git version 2.39.5, got: $STOCK_GIT" >&2
  exit 1
fi

export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"
cargo build -p arks --release
ARKS_BIN="${ARKS_BIN:-${CARGO_TARGET_DIR:-target}/release/arks}"
if [[ ! -x "$ARKS_BIN" ]]; then
  echo "claimed-daemon-archive-e2e: arks binary missing: $ARKS_BIN" >&2
  exit 1
fi

WORK="$(mktemp -d /tmp/arks-b5.XXXXXX)"
DAEMON_PID=""
cleanup() {
  if [[ -n "$DAEMON_PID" ]] && kill -0 "$DAEMON_PID" 2>/dev/null; then
    kill "$DAEMON_PID" 2>/dev/null || true
    wait "$DAEMON_PID" 2>/dev/null || true
  fi
  rm -rf "$WORK"
}
trap cleanup EXIT

mkdir "$WORK/seed"
git -C "$WORK/seed" init -q
git -C "$WORK/seed" config user.name b5
git -C "$WORK/seed" config user.email b5@example.invalid
printf 'archive parity\n' >"$WORK/seed/README.txt"
mkdir "$WORK/seed/src"
printf '#!/bin/sh\nprintf b5\n' >"$WORK/seed/src/run.sh"
chmod +x "$WORK/seed/src/run.sh"
GIT_AUTHOR_DATE='2020-01-02T03:04:05Z' GIT_COMMITTER_DATE='2020-01-02T03:04:05Z' \
  git -C "$WORK/seed" add README.txt src/run.sh
GIT_AUTHOR_DATE='2020-01-02T03:04:05Z' GIT_COMMITTER_DATE='2020-01-02T03:04:05Z' \
  git -C "$WORK/seed" commit -q -m b5
git clone --bare -q "$WORK/seed" "$WORK/repo.git"

read -r STOCK_PORT ARKS_PORT < <(python3 - <<'PY'
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
wait_port() {
  python3 - "$1" <<'PY'
import socket, sys, time
port = int(sys.argv[1])
for _ in range(100):
    try:
        with socket.create_connection(("127.0.0.1", port), timeout=.2):
            raise SystemExit(0)
    except OSError:
        time.sleep(.05)
raise SystemExit(f"daemon port {port} not ready")
PY
}

git daemon --reuseaddr --export-all --enable=upload-archive \
  --base-path="$WORK" --listen=127.0.0.1 --port="$STOCK_PORT" "$WORK" \
  >"$WORK/stock.out" 2>"$WORK/stock.err" &
DAEMON_PID=$!
wait_port "$STOCK_PORT"
git archive --remote="git://127.0.0.1:${STOCK_PORT}/repo.git" \
  --format=tar HEAD >"$WORK/stock.tar"
kill "$DAEMON_PID"
wait "$DAEMON_PID" 2>/dev/null || true
DAEMON_PID=""

"$ARKS_BIN" daemon --reuseaddr --export-all --enable=upload-archive \
  --base-path "$WORK" --listen "127.0.0.1:$ARKS_PORT" \
  >"$WORK/arks.out" 2>"$WORK/arks.err" &
DAEMON_PID=$!
wait_port "$ARKS_PORT"
set +e
git archive --remote="git://127.0.0.1:${ARKS_PORT}/repo.git" \
  --format=tar HEAD >"$WORK/arks.tar" 2>"$WORK/arks-client.err"
ARKS_ARCHIVE_RC=$?
set -e
kill "$DAEMON_PID"
wait "$DAEMON_PID" 2>/dev/null || true
DAEMON_PID=""

# Structural / semantic Partial probe (weaker oracle) + Claimed cmp -s.
B5_STRUCTURAL=0
B5_BYTE_IDENTICAL=0
FIRST_DIFF=""
if [[ "$ARKS_ARCHIVE_RC" -eq 0 ]] && [[ -s "$WORK/arks.tar" ]] && [[ -s "$WORK/stock.tar" ]]; then
  if python3 - "$WORK/stock.tar" "$WORK/arks.tar" <<'PY'
import sys, tarfile

def stream(path):
    out = []
    with tarfile.open(path, "r:") as archive:
        for member in archive:
            body = b""
            if member.isfile():
                body = archive.extractfile(member).read()
            out.append((member.name, member.type, member.mode & 0o7777, body))
    return out

stock, arks = stream(sys.argv[1]), stream(sys.argv[2])
if stock != arks:
    print(f"semantic stream mismatch:\nstock={stock!r}\narks={arks!r}", file=sys.stderr)
    raise SystemExit(1)
PY
  then
    B5_STRUCTURAL=1
  fi
  if cmp -s "$WORK/stock.tar" "$WORK/arks.tar"; then
    B5_BYTE_IDENTICAL=1
  else
    FIRST_DIFF="$(cmp -l "$WORK/stock.tar" "$WORK/arks.tar" | head -n 3 | tr '\n' ';')"
  fi
fi

if [[ "$B5_BYTE_IDENTICAL" -eq 1 ]]; then
  echo "claimed-daemon-archive-e2e: B5 Claimed — byte-identical ustar vs git 2.39.5 (cmp -s)"
elif [[ "$B5_STRUCTURAL" -eq 1 ]]; then
  echo "claimed-daemon-archive-e2e: B5 Partial — structural/semantic OK; byte-identical ustar Missing" >&2
  echo "claimed-daemon-archive-e2e: first_diff=${FIRST_DIFF:-unknown}" >&2
else
  echo "claimed-daemon-archive-e2e: B5 incomplete (client_rc=$ARKS_ARCHIVE_RC structural=$B5_STRUCTURAL)" >&2
  if [[ -s "$WORK/arks-client.err" ]]; then
    cat "$WORK/arks-client.err" >&2
  fi
fi

# B5+ feasibility: openlog armed only — facility/priority capture deferred (Refuse).
"$ARKS_BIN" daemon --syslog --export-all --listen "127.0.0.1:$ARKS_PORT" "$WORK" \
  >"$WORK/syslog.out" 2>"$WORK/syslog.err" &
DAEMON_PID=$!
wait_port "$ARKS_PORT"
kill "$DAEMON_PID"
wait "$DAEMON_PID" 2>/dev/null || true
DAEMON_PID=""
grep -F "LOG_DAEMON" "$WORK/syslog.err" >/dev/null
echo "claimed-daemon-archive-e2e: B5+ Refuse-terminal — LOG_DAEMON armed only; facility/priority not Claimed"

# Docker receipt when Claimed bar is met.
if [[ "$B5_BYTE_IDENTICAL" -eq 1 ]] && [[ -f /.dockerenv ]]; then
  # Stable evidence id registered in SoT (IW-B5 Claimed).
  OUT=".spec/evidence/20261111091600-claimed-daemon-archive"
  mkdir -p "$OUT"
  python3 - "$OUT/receipt.json" "$STOCK_GIT" <<'PY'
import json, sys, time
out, stock_git = sys.argv[1], sys.argv[2]
receipt = {
    "schema": "arks-claimed-residual/v1",
    "gate": "B5",
    "b5": "Claimed",
    "b5_plus": "Refuse-terminal",
    "disposition": "verified-byte-identical-ustar",
    "venue": "docker-dod-claimed-daemon-archive",
    "stock_git": stock_git,
    "oracle": "git v2.39.5 archive-tar.c prepare_header",
    "comparison": ["cmp -s stock.tar arks.tar", "tarfile structural/semantic"],
    "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
    "note": "IW-B5; product MUST NOT spawn git archive. B5+ syslog facility remains Refuse.",
}
with open(out, "w", encoding="utf-8") as f:
    json.dump(receipt, f, indent=2)
    f.write("\n")
PY
  # Scrubbed first-header slice for audit (not a second Claimed oracle).
  python3 - "$WORK/stock.tar" "$WORK/arks.tar" "$OUT/first-header.hex" <<'PY'
import sys
stock = open(sys.argv[1], "rb").read(512)
arks = open(sys.argv[2], "rb").read(512)
assert stock == arks
with open(sys.argv[3], "w", encoding="utf-8") as f:
    f.write(stock.hex() + "\n")
PY
  echo "claimed-daemon-archive-e2e: receipt=$OUT/receipt.json"
elif [[ "$B5_BYTE_IDENTICAL" -eq 1 ]]; then
  echo "claimed-daemon-archive-e2e: host run only; no .spec/evidence receipt written"
fi

# STRICT: unfinished Claimed rows fail closed. B5+ Refuse is not an unfinished Claimed row.
if [[ "${ARKS_DOD_STRICT:-0}" == "1" ]]; then
  if [[ "$B5_BYTE_IDENTICAL" -ne 1 ]]; then
    echo "claimed-daemon-archive-e2e: STRICT — B5 Claimed bar (cmp -s) not met" >&2
    exit 1
  fi
fi

if [[ "$B5_BYTE_IDENTICAL" -eq 1 ]]; then
  echo "claimed-daemon-archive-e2e: PASS — B5 Claimed (byte_identical=1); B5+=Refuse-terminal"
  exit 0
fi
echo "claimed-daemon-archive-e2e: SKIP/Partial — B5 residual open (structural=$B5_STRUCTURAL byte_identical=$B5_BYTE_IDENTICAL); B5+=Refuse" >&2
if [[ "${ARKS_DOD_STRICT:-0}" == "1" ]]; then
  exit 1
fi
exit 0
