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
# History-family extras (argv symmetry A2): a three-commit side history
# rooted outside every ref and kept alive by a reflog without a ref
# (.git/logs/refs/fixture/history), so refs, `--all`, the index and the
# work tree are unchanged. Built with a temporary index:
#   H1 base files; H2 renames (exact + inexact), a copy, a mode change, a
#   type change, whitespace-only and blank-line edits, a function-body
#   edit, trailing whitespace; H3 a complete rewrite.
hx() {
  local idx="$PWD/.git/fixture-history.index" tree spec m rest p body b
  rm -f "$idx"
  for spec in "$@"; do
    # spec: mode:path:content (content is a printf format)
    m="${spec%%:*}"; rest="${spec#*:}"; p="${rest%%:*}"; body="${rest#*:}"
    b=$(printf "$body" | git hash-object -w --stdin)
    GIT_INDEX_FILE="$idx" git update-index --add --cacheinfo "$m,$b,$p"
  done
  tree=$(GIT_INDEX_FILE="$idx" git write-tree)
  rm -f "$idx"
  printf '%s' "$tree"
}
hc() { # <tree> <message> [<parent>]
  t=$((t + 3600))
  GIT_AUTHOR_DATE="$t +0000" GIT_COMMITTER_DATE="$t +0000" \
    git commit-tree "$1" -m "$2" ${3:+-p "$3"}
}
l20='l1\nl2\nl3\nl4\nl5\nl6\nl7\nl8\nl9\nl10\nl11\nl12\nl13\nl14\nl15\nl16\nl17\nl18\nl19\nl20\n'
fc1='int one(void)\n{\n\treturn 1;\n}\n\nint two(void)\n{\n\tint x = 2;\n\treturn x;\n}\n\nint three(void)\n{\n\treturn 3;\n}\n'
fc2="${fc1/x = 2/x = 22}"
rw1=$(for i in $(seq 1 30); do printf 'original line number %s\\n' "$i"; done)
rw2=$(for i in $(seq 1 30); do printf 'rewritten content row %s\\n' "$i"; done)
h_common=("100644:same-renamed.txt:same\ncontent\nhere\n"
  "100644:code-copy.c:$fc1" "100644:ws.txt:a  b\n        c\nd e \nF\n" "100755:run.sh:echo hi\n"
  "120000:kind.txt:same.txt" "100644:blank.txt:x\n\n\ny\n" "100644:trail.txt:ok\nbad \n \tmixed\n"
  "100644:moved.txt:${l20/l7/L7}")
h1=$(hc "$(hx "100644:big.txt:$l20" "100644:same.txt:same\ncontent\nhere\n" \
  "100644:code.c:$fc1" "100644:ws.txt:a b\n\tc\nd  e\nf\n" "100644:run.sh:echo hi\n" \
  "100644:kind.txt:plain\n" "100644:blank.txt:x\ny\n" \
  "100644:rewrite.txt:$rw1")" "history: base")
h2=$(hc "$(hx "${h_common[@]}" "100644:code.c:$fc2" "100644:rewrite.txt:$rw1")" \
  "history: renames, copies, modes" "$h1")
h3=$(hc "$(hx "${h_common[@]}" "100644:code.c:$fc2" "100644:rewrite.txt:$rw2")" \
  "history: rewrite" "$h2")
# H4 merges H3 with a side commit S (from H1); code.c is resolved by hand.
fc3="${fc1/x = 2/x = 20}"; fc3="${fc3/return 3;/return 33;}"
fc4="${fc1/x = 2/x = 222}"; fc4="${fc4/return 3;/return 33;}"
hs=$(hc "$(hx "100644:big.txt:$l20" "100644:same.txt:same\ncontent\nhere\n" \
  "100644:code.c:$fc3" "100644:ws.txt:a b\n\tc\nd  e\nf\n" "100644:run.sh:echo hi\n" \
  "100644:kind.txt:plain\n" "100644:blank.txt:x\ny\n" "100644:side.txt:side\n" \
  "100644:rewrite.txt:$rw1")" "history: side" "$h1")
h4t=$(hx "${h_common[@]}" "100644:code.c:$fc4" "100644:rewrite.txt:$rw2" \
  "100644:side.txt:side\n")
t=$((t + 3600))
h4=$(GIT_AUTHOR_DATE="$t +0000" GIT_COMMITTER_DATE="$t +0000" \
  git commit-tree "$h4t" -p "$h3" -p "$hs" -m "history: merge side")
mkdir -p .git/logs/refs/fixture
printf '%s %s C O Mitter <committer@example.com> %s +0000\tfixture: history\n' \
  0000000000000000000000000000000000000000 "$h4" "$t" > .git/logs/refs/fixture/history
# Family commit extras: a separate clean repository nested inside .git (so the
# main repository's refs, objects, index and status are unchanged) whose
# branches diverge from `base`: `topic` (other files) and `lineone` (another
# line of a.txt) merge cleanly into `main`, `clash` conflicts with it.
# Corpus lines reach it with `-C .git/side`.
mkdir -p .git/side && (
  cd .git/side
  git init -q -b main .
  git config user.name "C O Mitter"; git config user.email committer@example.com
  git config commit.gpgsign false
  printf 'one\ntwo\nthree\n' > a.txt; printf 'base\n' > b.txt
  git add . && c -m "side: base"
  git tag base
  git checkout -q -b topic
  printf 'base\ntopic\n' > b.txt; git add b.txt && c -m "topic: extend b"
  printf 'new\n' > c.txt; git add c.txt && c -m "topic: add c"
  git checkout -q -b clash base
  printf 'one\ntwo\n3\n' > a.txt; git add a.txt && c -m "clash: number three"
  git checkout -q -b lineone base
  printf 'ONE\ntwo\nthree\n' > a.txt; git add a.txt && c -m "lineone: shout one"
  git checkout -q main
  printf 'one\ntwo\nTHREE\n' > a.txt; git add a.txt && c -m "main: shout three"
)
# A commit message with a trailer block for interpret-trailers lines.
printf 'subject\n\nbody text\n\nSigned-off-by: A U Thor <author@example.com>\nFixes: 1\n' > .git/trailer-msg.txt
# Patches for apply lines: one that applies to src/a.txt (index and work
# tree), the same without a/ b/ prefixes (-p0), one creating a file with a
# whitespace error, and one whose second hunk does not apply (--reject).
printf 'diff --git a/src/a.txt b/src/a.txt\n--- a/src/a.txt\n+++ b/src/a.txt\n@@ -6,3 +6,3 @@\n zeta\n-eta\n+ETA\n theta\n' > .git/a.patch
printf 'diff --git src/a.txt src/a.txt\n--- src/a.txt\n+++ src/a.txt\n@@ -6,3 +6,3 @@\n zeta\n-eta\n+ETA\n theta\n' > .git/p0.patch
printf 'diff --git a/docs/n.md b/docs/n.md\nnew file mode 100644\n--- /dev/null\n+++ b/docs/n.md\n@@ -0,0 +1,2 @@\n+hello \n+world\n' > .git/new.patch
printf 'diff --git a/src/a.txt b/src/a.txt\n--- a/src/a.txt\n+++ b/src/a.txt\n@@ -1,2 +1,2 @@\n-alpha\n+ALPHA\n beta\n@@ -7,2 +7,2 @@\n-nothing\n+here\n theta\n' > .git/bad.patch
# Mailboxes for am lines in the side repository: two clean patches and one
# that conflicts with main.
(
  cd .git/side
  git format-patch -q --stdout base..topic > .git/topic.mbox
  git format-patch -q --stdout -1 clash > .git/clash.mbox
)
# Family A3 helpers extras (family-a3-helpers.txt `-C .git/strat...` lines):
# two clean repositories nested inside .git, so the fixture's own refs,
# objects, index and status are unchanged. `.git/strat` has branches that
# diverge from `base` (`topic`, `lineone`, `other` merge cleanly into main;
# `clash` conflicts with it), an unrelated `proj` history and main carrying
# proj's first commit under lib/ (subtree merges). Stat checks are minimal so
# stock git does not see the copied fixture as stat-dirty. `.git/stratc` is a
# copy left in the middle of a conflicted merge-resolve of `clash`.
mkdir -p .git/strat && (
  cd .git/strat
  git init -q -b main .
  git config user.name "C O Mitter"; git config user.email committer@example.com
  git config commit.gpgsign false; git config core.checkStat minimal
  git config core.trustctime false
  printf 'one\ntwo\nthree\n' > a.txt; printf 'base\n' > b.txt
  git add . && c -m "strat: base"
  git tag base
  git checkout -q -b topic
  printf 'base\ntopic\n' > b.txt; git add b.txt && c -m "topic: extend b"
  printf 'new\n' > c.txt; git add c.txt && c -m "topic: add c"
  git checkout -q -b clash base
  printf 'one\ntwo\n3\n' > a.txt; git add a.txt && c -m "clash: number three"
  git checkout -q -b lineone base
  printf 'ONE\ntwo\nthree\n' > a.txt; git add a.txt && c -m "lineone: shout one"
  git checkout -q -b other base
  printf 'other\n' > d.txt; git add d.txt && c -m "other: add d"
  git checkout -q --orphan proj
  git rm -rqf .
  printf 'p1\n' > p.txt; printf 'q\n' > q.txt; git add p.txt q.txt && c -m "proj: one"
  git checkout -q main
  printf 'one\ntwo\nTHREE\n' > a.txt; git add a.txt && c -m "main: shout three"
  git read-tree --prefix=lib/ -u proj && c -m "main: add proj under lib"
  git checkout -q proj
  printf 'p2\n' > p.txt; git add p.txt && c -m "proj: two"
  git checkout -q main
)
cp -a .git/strat .git/stratc
(cd .git/stratc && git merge-resolve base -- HEAD clash >/dev/null 2>&1 || :)
# External tool bridges (A3 tools): a nested repository `.git/conflict` left in
# a conflicted merge (`m.txt` and `n.txt` both changed, `gone.txt` modified on
# main and deleted on `other`, `ok.txt` merged cleanly) for mergetool lines,
# and a quilt series under `.git/side/.git/quilt` for quiltimport lines.
mkdir -p .git/conflict && (
  cd .git/conflict
  git init -q -b main .
  git config user.name "C O Mitter"; git config user.email committer@example.com
  git config commit.gpgsign false
  printf 'one\ntwo\nthree\n' > m.txt; printf 'red\ngreen\n' > n.txt
  printf 'keep\n' > gone.txt; printf 'base\n' > ok.txt
  git add . && c -m "conflict: base"
  git checkout -q -b other
  printf 'one\nTWO (other)\nthree\n' > m.txt; printf 'red\nGREEN (other)\n' > n.txt
  git rm -q gone.txt; printf 'base\nother\n' > ok.txt
  git add . && c -m "other: edit and delete"
  git checkout -q main
  printf 'one\nTWO (main)\nthree\n' > m.txt; printf 'red\nGREEN (main)\n' > n.txt
  printf 'keep\nmain\n' > gone.txt
  git add . && c -m "main: edit"
  t=$((t + 3600)); GIT_AUTHOR_DATE="$t +0000" GIT_COMMITTER_DATE="$t +0000" \
    git merge -q other -m "Merge other" >/dev/null 2>&1 || true
)
q=.git/side/.git/quilt
mkdir -p "$q/sub"
printf 'From: Q U Ilt <quilt@example.com>\nSubject: quilt: extend b\n\nAdd a line to b.\n---\n' > "$q/one.patch"
printf 'diff --git a/b.txt b/b.txt\n--- a/b.txt\n+++ b/b.txt\n@@ -1 +1,2 @@\n base\n+quilt one\n' >> "$q/one.patch"
printf 'Second patch without a mail header.\n\nIt adds q.txt.\n\n' > "$q/sub/two.diff"
printf 'diff --git a/q.txt b/q.txt\nnew file mode 100644\n--- /dev/null\n+++ b/q.txt\n@@ -0,0 +1 @@\n+quilt two\n' >> "$q/sub/two.diff"
printf '# quilt series\none.patch\nsub/two.diff -p1\n' > "$q/series"
printf 'one.patch\nmissing.patch\n' > "$q/series-missing"
# Range-diff gap extras (family-gap-range-diff.txt `-C .git/rd` lines): a clean
# repository nested inside .git (the fixture's own refs, objects, index and
# status are unchanged) with two versions of a series on `base`. `old` and
# `new` share an identical commit, a reworded one, a re-edited one, a dropped
# and an added one plus renames, mode changes, deletions and a binary file;
# `solo` holds one commit against three on `old` (stock git's assignment
# leaves them unpaired); notes on both default and `refs/notes/foo` refs.
mkdir -p .git/rd && (
  cd .git/rd
  git init -q -b main .
  git config user.name "C O Mitter"; git config user.email committer@example.com
  git config commit.gpgsign false
  printf 'int main(void)\n{\n\tone();\n\ttwo();\n\tthree();\n\tfour();\n\tfive();\n\tsix();\n}\n' > a.c
  printf '1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n' > n.txt
  printf 'mv1\nmv2\nmv3\nmv4\nmv5\n' > m.txt
  printf 'sh\n' > s.sh; printf 'del\n' > d.txt
  git add . && c -m "rd: base"
  git tag base
  git checkout -q -b old
  printf 'int main(void)\n{\n\tone();\n\tTWO();\n\tthree();\n\tfour();\n\tfive();\n\tsix();\n}\n' > a.c
  git add . && c -m "shout two" -m "Body old."
  printf '1\n2\n3\n4\nfive\n6\n7\n8\n9\n10\n' > n.txt; git add . && c -m "five words"
  printf 'x\n' > x.txt; git add . && c -m "add x"
  git mv m.txt m2.txt; chmod +x s.sh; git rm -q d.txt; git add . && c -m "rename and mode"
  printf 'gone\n' > g.txt; git add . && c -m "dropped later"
  git checkout -q -b new base
  printf 'int main(void)\n{\n\tone();\n\tTWO();\n\tthree();\n\tfour();\n\tfive();\n\tsix();\n}\n' > a.c
  git add . && c -m "shout two" -m "Body new."
  printf 'x\n' > x.txt; git add . && c -m "add x"
  printf '1\n2\n3\n4\nFIVE\n6\n7\n8\n9\n10\n' > n.txt; git add . && c -m "five words"
  git mv m.txt m3.txt; chmod +x s.sh; git rm -q d.txt; git add . && c -m "rename and mode"
  printf 'bin\0ary\n' > b.bin; printf 'y\ny\n' > y.txt; git add . && c -m "brand new"
  git checkout -q -b solo base
  printf 'y\ny\n' > y.txt; git add . && c -m "solo commit"
  git checkout -q main
  GIT_COMMITTER_DATE="$t +0000" git notes add -m "old note" old~1
  GIT_COMMITTER_DATE="$t +0000" git notes --ref=foo add -m "foo note" old~2
  GIT_COMMITTER_DATE="$t +0000" git notes --ref=foo add -m "foo note two" new~2
)
