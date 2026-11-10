#!/usr/bin/env bash
# Claimed residual gates B5/B5+ — daemon archive parity + syslog facility.
# SoT: .spec/sot/arks-claimed-residual-gates.sot.md
# verify-directory-conventions: default SKIP/Partial honesty; ARKS_DOD_STRICT=1 fail-closed.
# Product MUST NOT spawn git. Stock git 2.39.5 is oracle only.
# STOP: byte-identical ustar vs git-daemon is NOT Claimed until evidence; do not invent PASS.
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

# Structural / semantic Partial probe (not Claimed).
B5_STRUCTURAL=0
B5_BYTE_IDENTICAL=0
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
  fi
fi

# Honesty: SoT holds B5 as Partial. Known mismatch class (501009): ustar header
# encoding (chksum digits + ASCII0 vs NUL device fields). Do not invent Claimed
# from structural greens or opportunistic cmp; deliberate Claimed wave only.
if [[ "$B5_BYTE_IDENTICAL" -eq 1 ]]; then
  echo "claimed-daemon-archive-e2e: B5 byte-identical observed this run — SoT still Partial (no invent Claimed)"
elif [[ "$B5_STRUCTURAL" -eq 1 ]]; then
  echo "claimed-daemon-archive-e2e: B5 Partial — structural/semantic OK; byte-identical ustar Missing"
  echo "claimed-daemon-archive-e2e: mismatch class=ustar header encoding (chksum @off151 + devmajor/minor ASCII0 vs NUL); see SoT B5"
else
  echo "claimed-daemon-archive-e2e: B5 incomplete (client_rc=$ARKS_ARCHIVE_RC structural=$B5_STRUCTURAL)" >&2
  if [[ -s "$WORK/arks-client.err" ]]; then
    cat "$WORK/arks-client.err" >&2
  fi
fi

# B5+ feasibility: openlog armed only — facility/priority capture deferred.
"$ARKS_BIN" daemon --syslog --export-all --listen "127.0.0.1:$ARKS_PORT" "$WORK" \
  >"$WORK/syslog.out" 2>"$WORK/syslog.err" &
DAEMON_PID=$!
wait_port "$ARKS_PORT"
kill "$DAEMON_PID"
wait "$DAEMON_PID" 2>/dev/null || true
DAEMON_PID=""
grep -F "LOG_DAEMON" "$WORK/syslog.err" >/dev/null
echo "claimed-daemon-archive-e2e: B5+ Partial — LOG_DAEMON armed; facility/priority capture deferred"

# STRICT: B5 Claimed bar + B5+ remain fail-closed. Default: SKIP/Partial exit 0.
if [[ "${ARKS_DOD_STRICT:-0}" == "1" ]]; then
  echo "claimed-daemon-archive-e2e: STRICT — B5 Claimed (byte-identical + SoT graduate) and B5+ not evidenced" >&2
  exit 1
fi
echo "claimed-daemon-archive-e2e: SKIP/Partial — B5 residual open (structural=$B5_STRUCTURAL byte_identical=$B5_BYTE_IDENTICAL); B5+=Partial"
