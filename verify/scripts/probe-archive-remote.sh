#!/usr/bin/env bash
set -euo pipefail
WORK=$(mktemp -d /tmp/b5probe.XXXXXX)
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
PORT=$(python3 - <<'PY'
import socket
s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1]); s.close()
PY
)
git daemon --reuseaddr --export-all --enable=upload-archive \
  --base-path="$WORK" --listen=127.0.0.1 --port="$PORT" "$WORK" \
  >"$WORK/daemon.out" 2>"$WORK/daemon.err" &
DPID=$!
python3 - "$PORT" <<'PY'
import socket,sys,time
p=int(sys.argv[1])
for _ in range(100):
  try:
    socket.create_connection(("127.0.0.1",p),timeout=.2).close(); raise SystemExit
  except OSError: time.sleep(.05)
raise SystemExit(1)
PY
git archive --remote="git://127.0.0.1:${PORT}/repo.git" --format=tar HEAD >"$WORK/remote.tar"
kill "$DPID"; wait "$DPID" 2>/dev/null || true
echo "=== remote archive members ==="
python3 - <<PY
import tarfile
with tarfile.open("$WORK/remote.tar","r:") as a:
  for m in a:
    print(repr(m.name), repr(m.type), m.mode, oct(m.mode), "size", m.size)
PY
echo "=== remote head bytes ==="
od -An -tx1 -N 64 "$WORK/remote.tar"
rm -rf "$WORK"
