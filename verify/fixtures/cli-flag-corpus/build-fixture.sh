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
# Remote family extras (family-remote.txt): a bare upstream inside .git and a
# configured `origin` remote pointing at it, with no remote-tracking refs, so
# no existing ref, commit or decoration changes. Upstream differs from the
# fixture: main has one more commit, `topic` is new (from t0), `feature` and
# tag t0 are absent, v2 (annotated) and `lt` (lightweight) are new tags.
u=.git/upstream.git
git clone -q --bare --no-local . "$u"
ui="$d/.git/upstream.index"
up_commit() { # <parent> <path> <content> <message>
  GIT_INDEX_FILE="$ui" git --git-dir="$u" read-tree "$1"
  b=$(printf '%s\n' "$3" | git --git-dir="$u" hash-object -w --stdin)
  GIT_INDEX_FILE="$ui" git --git-dir="$u" update-index --add --cacheinfo "100644,$b,$2"
  tr=$(GIT_INDEX_FILE="$ui" git --git-dir="$u" write-tree)
  t=$((t + 3600))
  GIT_AUTHOR_DATE="$t +0000" GIT_COMMITTER_DATE="$t +0000" \
    git --git-dir="$u" commit-tree "$tr" -p "$1" -m "$4"
}
m=$(up_commit main up.txt upstream "upstream: add up.txt")
git --git-dir="$u" update-ref refs/heads/main "$m"
tp=$(up_commit t0 topic.txt topic "topic: add topic.txt")
git --git-dir="$u" update-ref refs/heads/topic "$tp"
GIT_COMMITTER_DATE="$t +0000" git --git-dir="$u" tag -a v2 -m "release v2" main
git --git-dir="$u" tag lt topic
git --git-dir="$u" tag -d t0 >/dev/null
git --git-dir="$u" branch -q -D feature
rm -f "$ui"
git config remote.origin.url "$u"
git config remote.origin.fetch '+refs/heads/*:refs/remotes/origin/*'
# Bundles for `bundle verify / list-heads / unbundle` lines: one with a
# prerequisite (t0..main) and one complete (feature plus tag v1).
git bundle create -q .git/range.bundle t0..main
git bundle create -q .git/full.bundle feature v1
# Submodule extras (family-remote.txt `-C .git/sm/...` lines), all under
# .git/sm so the fixture's own refs, index and status do not change:
# `lib` (main: two commits, tag lv1 at the first, branch dev), `super`
# recording lib at lv1 while its checkout sits on lib's main (status `+`),
# and `clone`, a plain clone of super whose submodule is not initialised.
sm="$d/.git/sm"
mkdir -p "$sm"
git init -q -b main "$sm/lib"
(cd "$sm/lib"
  printf 'lib one\n' > l.txt && git add l.txt && c -m "lib: one" && git tag lv1
  git branch dev
  printf 'lib two\n' > l.txt && c -am "lib: two")
git init -q -b main "$sm/super"
(cd "$sm/super"
  printf 'super\n' > s.txt && git add s.txt && c -m "super: init"
  git -c protocol.file.allow=always submodule add -q ../lib lib 2>/dev/null
  git -C lib checkout -q lv1 && git add lib && c -m "super: add lib at lv1"
  git -C lib checkout -q main)
(cd "$sm" && git clone -q super clone)
# Plumbing family extras (no new reachable commits): one pack holding the
# objects of the first commit (they stay loose too, for prune-packed), a fresh
# dangling blob and an old dangling blob (for prune --expire).
git rev-list --objects "$(git rev-list --max-parents=0 HEAD)" | git pack-objects -q .git/objects/pack/pack >/dev/null
printf 'dangling fresh\n' | git hash-object -w --stdin >/dev/null
old=$(printf 'dangling old\n' | git hash-object -w --stdin)
touch -d @1600000000 ".git/objects/${old:0:2}/${old:2}"
