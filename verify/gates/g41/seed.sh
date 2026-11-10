#!/usr/bin/env bash
# Seed a bare repo into the shared Apache DocumentRoot volume for G41.
set -euo pipefail
ROOT="${ARKS_G41_DOCROOT:-/git-data}"
mkdir -p "$ROOT"
WORK="$(mktemp -d)"
export GIT_AUTHOR_NAME=arks GIT_AUTHOR_EMAIL=arks@example.com
export GIT_COMMITTER_NAME=arks GIT_COMMITTER_EMAIL=arks@example.com
export GIT_AUTHOR_DATE='1000000000 +0000' GIT_COMMITTER_DATE='1000000000 +0000'
git init -b main "$WORK"
printf 'hello apache dav\n' >"$WORK/README"
git -C "$WORK" add README
git -C "$WORK" commit -m c1
rm -rf "$ROOT/repo.git"
git clone --bare "$WORK" "$ROOT/repo.git"
git -C "$ROOT/repo.git" --bare update-server-info
# Apache daemon user needs write for DAV LOCK/PUT/DELETE.
chmod -R a+rwX "$ROOT/repo.git"
echo "G41 seed: $ROOT/repo.git ready"
rm -rf "$WORK"
