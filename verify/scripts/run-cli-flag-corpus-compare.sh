#!/usr/bin/env bash
# CLI flag corpus peer compare (HOST ADVISORY unless run in the clean-room image).
# Plan: .spec/plan/20261119210000-cloud-dropin-remaining-rungs.plan.md (C7a/C7b).
#
# Usage: run-cli-flag-corpus-compare.sh <arks-bin> <corpus.txt> <out-dir>
# For each corpus line: copy the fixture twice, run the line with arks (personality git) and
# with stock git (peer oracle only), compare exit code + stdout, then compare repository state
# (refs, HEAD, index, worktree status) read back with stock git. SAME requires all to match.
set -uo pipefail
ARKS="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"
CORPUS="$(cd "$(dirname "$2")" && pwd)/$(basename "$2")"
OUT="$3"; mkdir -p "$OUT"; OUT="$(cd "$OUT" && pwd)"
HERE="$(cd "$(dirname "$0")" && pwd)"
FIX_SH="$HERE/../fixtures/cli-flag-corpus/build-fixture.sh"
WORK="$(mktemp -d)"; trap 'rm -rf "$WORK"' EXIT
bash "$FIX_SH" "$WORK/fixture" >/dev/null
export GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null GIT_PAGER=cat PAGER=cat TZ=UTC
export GIT_AUTHOR_NAME="A U Thor" GIT_AUTHOR_EMAIL=author@example.com
export GIT_COMMITTER_NAME="C O Mitter" GIT_COMMITTER_EMAIL=committer@example.com
export GIT_AUTHOR_DATE="1700100000 +0000" GIT_COMMITTER_DATE="1700100000 +0000"
export GIT_EDITOR=true
state() {
  (cd "$1" && {
    git for-each-ref --format='%(refname) %(objectname)' 2>&1
    echo "HEAD $(git symbolic-ref -q HEAD || git rev-parse HEAD)"
    git ls-files -s 2>&1
    git status --porcelain=v1 -uall 2>&1
    git stash list --format='%gs' 2>&1
  })
}
n=0; same=0; rows=()
while IFS= read -r line || [[ -n "$line" ]]; do
  [[ -z "$line" || "$line" == \#* ]] && continue
  n=$((n + 1)); id=$(printf 'L%03d' "$n")
  rm -rf "$WORK/a" "$WORK/g"; cp -a "$WORK/fixture" "$WORK/a"; cp -a "$WORK/fixture" "$WORK/g"
  eval "set -- $line"
  ao="$(cd "$WORK/a" && ARKS_PERSONALITY=git "$ARKS" "$@" 2>"$OUT/$id.arks.err")"; ac=$?
  go="$(cd "$WORK/g" && git "$@" 2>"$OUT/$id.git.err")"; gc=$?
  as="$(state "$WORK/a")"; gs="$(state "$WORK/g")"
  verdict=SAME; why=""
  [[ $ac -ne $gc ]] && { verdict=DIFF; why="exit $ac!=$gc"; }
  [[ "$ao" != "$go" ]] && { verdict=DIFF; why="${why:+$why; }stdout"; }
  [[ "$as" != "$gs" ]] && { verdict=DIFF; why="${why:+$why; }state"; }
  if [[ $verdict == DIFF ]]; then
    printf '%s\n' "$ao" >"$OUT/$id.arks.out"; printf '%s\n' "$go" >"$OUT/$id.git.out"
    printf '%s\n' "$as" >"$OUT/$id.arks.state"; printf '%s\n' "$gs" >"$OUT/$id.git.state"
  else
    same=$((same + 1)); rm -f "$OUT/$id.arks.err" "$OUT/$id.git.err"
  fi
  printf '%s\t%s\t%s\t%s\n' "$id" "$verdict" "$line" "$why" >>"$OUT/results.tsv.tmp"
done <"$CORPUS"
mv "$OUT/results.tsv.tmp" "$OUT/results.tsv"
printf '{"corpus":"%s","git":"%s","lines":%d,"same":%d,"diff":%d}\n' \
  "$(basename "$CORPUS")" "$(git --version)" "$n" "$same" "$((n - same))" >"$OUT/summary.json"
cat "$OUT/summary.json"
