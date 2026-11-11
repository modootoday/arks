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
echo "=== ls-tree ==="
git -C "$WORK/seed" ls-tree -r HEAD
echo "=== local archive members ==="
git -C "$WORK/seed" archive --format=tar HEAD >"$WORK/local.tar"
python3 - <<PY
import tarfile
with tarfile.open("$WORK/local.tar", "r:") as a:
    for m in a:
        print(repr(m.name), repr(m.type), m.mode, oct(m.mode))
PY
echo "=== first headers hex ==="
od -An -tx1 -N 1024 "$WORK/local.tar" | head -n 40
rm -rf "$WORK"
