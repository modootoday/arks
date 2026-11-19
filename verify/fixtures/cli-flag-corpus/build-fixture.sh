#!/usr/bin/env bash
# Deterministic fixture repo for the CLI flag corpus (stock git builds it; oracle use only).
# Usage: build-fixture.sh <dir>
set -euo pipefail
d="$1"
rm -rf "$d"; mkdir -p "$d"; cd "$d"
export GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
export GIT_AUTHOR_NAME="A U Thor" GIT_AUTHOR_EMAIL=author@example.com
export GIT_COMMITTER_NAME="C O Mitter" GIT_COMMITTER_EMAIL=committer@example.com
t=1700000000
c() { t=$((t + 3600)); GIT_AUTHOR_DATE="$t +0000" GIT_COMMITTER_DATE="$t +0000" git commit -q "$@"; }
git init -q -b main .
git config user.name "C O Mitter"; git config user.email committer@example.com
git config commit.gpgsign false; git config core.autocrlf false
printf 'readme\n' > README.md
mkdir -p src docs
printf 'alpha\nbeta\ngamma\ndelta\nepsilon\nzeta\neta\ntheta\n' > src/a.txt
printf 'one\ntwo\nthree\n' > src/b.txt
git add . && c -m "initial import"
git tag t0
printf 'alpha\nbeta\nGAMMA\ndelta\nepsilon\nzeta\neta\ntheta\niota\n' > src/a.txt
printf '# Docs\n\nSome docs.\n' > docs/x.md
git add . && c -m "update a, add docs" -m "Body line one.
Body line two."
git checkout -q -b feature
printf 'one\nTWO\nthree\nfour\n' > src/b.txt
git add . && c -m "feature: change b"
printf 'bin\0ary\n' > src/data.bin
git add . && c -m "feature: add binary"
git checkout -q main
printf 'readme\nmore readme\n' > README.md
git add . && c -m "main: extend readme"
GIT_COMMITTER_DATE="$((t + 60)) +0000" git tag -a v1 -m "release v1"
t=$((t + 3600)); GIT_AUTHOR_DATE="$t +0000" GIT_COMMITTER_DATE="$t +0000" git merge -q --no-ff feature -m "Merge branch 'feature'"
printf 'readme\nmore readme\nlast\n' > README.md
git add README.md && c -m "main: tail readme"
# Tier B extras (no new reachable commits, so tier A hashes are unchanged):
# an mbox from a throwaway branch, a diff patch, one stash entry, an ignore
# rule with an ignored file, and info/attributes.
git checkout -q -b tmp
printf 'one\nTWO\nthree\nfour\nfive\n' > src/b.txt
git add src/b.txt && c -m "tmp: extend b"
git format-patch -q --stdout -1 > .git/fixture.mbox
git checkout -q main && git branch -q -D tmp
git diff HEAD~1 HEAD > .git/fixture.patch
printf 'readme\nmore readme\nlast\nstashed\n' > README.md
t=$((t + 3600)); GIT_AUTHOR_DATE="$t +0000" GIT_COMMITTER_DATE="$t +0000" git stash push -q -m fixture-stash
printf '*.log\n' >> .git/info/exclude
printf 'debug\n' > debug.log
printf '*.bin binary\nsrc/*.txt label=src\n' > .git/info/attributes
# Working tree state: unstaged edit, staged new file, untracked file.
printf 'alpha\nbeta\nGAMMA\ndelta!\nepsilon\nzeta\neta\ntheta\niota\n' > src/a.txt
printf 'staged\n' > src/new.txt && git add src/new.txt
printf 'scratch\n' > untracked.txt
