#!/usr/bin/env bash
# Cloud agent drop-in host probe (HOST ADVISORY — not a clean-room receipt).
#
# Runs the agent option corpus from .spec/sot/cloud-agent-dropin.sot.md §O with
# `arks` (personality git) inside a hosted agent sandbox and compares stdout with
# stock git on the same clone (stock git = peer oracle only; product paths never
# spawn git). Also exercises proxy/CA transport (§T), merged-config identity +
# signing (§C/§S), fetch → FETCH_HEAD → merge, and optionally a push.
#
# Usage:
#   verify/scripts/run-cloud-dropin-host-probe.sh <arks-bin> <remote-url> <out-dir>
# Env:
#   ARKS_PROBE_PUSH_REF   optional refs/heads/... to push HEAD to (session branch only)
#
# Exit 0 when every MUST row passes; SAME/DIFF compare rows are recorded, not gated.
set -uo pipefail

ARKS="${1:?arks binary}"
URL="${2:?remote url}"
OUT="${3:?out dir}"
mkdir -p "$OUT"
OUT="$(cd "$OUT" && pwd)"
ARKS="$(cd "$(dirname "$ARKS")" && pwd)/$(basename "$ARKS")"
export ARKS_PERSONALITY=git
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

fails=0
rows=()
must() {
  local id="$1"; shift
  if "$@" >"$OUT/$id.out" 2>"$OUT/$id.err"; then
    rows+=("{\"id\":\"$id\",\"class\":\"must\",\"result\":\"PASS\"}")
  else
    rows+=("{\"id\":\"$id\",\"class\":\"must\",\"result\":\"FAIL\"}")
    fails=$((fails + 1))
  fi
}
same() {
  local id="$1"; shift
  local a g
  a="$("$ARKS" "$@" 2>&1)"
  g="$(git "$@" 2>&1)"
  printf '%s\n' "$a" >"$OUT/$id.arks.out"
  printf '%s\n' "$g" >"$OUT/$id.git.out"
  if [[ "$a" == "$g" ]]; then
    rows+=("{\"id\":\"$id\",\"class\":\"compare\",\"result\":\"SAME\"}")
  else
    rows+=("{\"id\":\"$id\",\"class\":\"compare\",\"result\":\"DIFF\"}")
  fi
}

cd "$WORK"
must T-CLONE "$ARKS" clone "$URL" repo
cd repo || exit 1
must T-FSCK git fsck --no-progress
must T-LS-REMOTE "$ARKS" ls-remote origin

# Read-only corpus vs peer.
same O-LOG-ONELINE log --oneline -5
same O-LOG-FORMAT log -5 --format='%H %h %T %P %an <%ae> %ad %aI %cn %ce %s'
same O-LOG-RANGE log --oneline HEAD~4..HEAD
same O-LOG-PRETTY log --pretty=format:%s -3
same O-LOG-PATH log --oneline -3 -- crates
same O-REV-PARSE rev-parse HEAD~2
same O-REV-PARSE-CARET rev-parse HEAD^
same O-SHOW-STAT show --stat --format=%s HEAD~1
same O-SHOW-NAME-ONLY show --name-only --format=%H HEAD~1
same O-DIFF-RANGE-STAT diff --stat HEAD~5 HEAD
same O-DIFF-RANGE-NAMES diff --name-status HEAD~3...HEAD
same O-BRANCH-CURRENT branch --show-current
same O-REMOTE-V remote -v
same O-CONFIG-USER config user.name

# Working-tree corpus.
first="$(git ls-files | head -1)"
echo "probe change" >>"$first"
echo "probe new" >probe-untracked.txt
same O-STATUS-SB status -sb
same O-STATUS-PORCELAIN status --porcelain
same O-DIFF diff
same O-DIFF-HEAD diff HEAD -- "$first"
same O-DIFF-STAT diff --stat
rm -f probe-untracked.txt

# Commit with merged-config identity (+ signing when commit.gpgsign is set).
must C-COMMIT-AM "$ARKS" commit -am "cloud drop-in probe commit"
author="$(git log -1 --format='%an <%ae>')"
expect="$(git config user.name) <$(git config user.email)>"
must C-IDENTITY test "$author" = "$expect"
if [[ "$(git config --bool commit.gpgsign 2>/dev/null)" == "true" ]]; then
  must S-GPGSIG sh -c 'git cat-file -p HEAD | grep -q "^gpgsig "'
fi
must C-FSCK-AFTER-COMMIT git fsck --no-progress

# fetch <branch> → FETCH_HEAD → merge --no-edit.
default_branch="$(git rev-parse --abbrev-ref HEAD)"
must W-CHECKOUT-B "$ARKS" checkout -B probe-topic HEAD~1
must W-FETCH-BRANCH "$ARKS" fetch origin "$default_branch"
must W-FETCH-HEAD test -s .git/FETCH_HEAD
must W-MERGE-FETCH-HEAD "$ARKS" merge --no-edit FETCH_HEAD
must W-PUSH-DRY-RUN "$ARKS" push --dry-run origin HEAD:refs/heads/probe-dry-run

if [[ -n "${ARKS_PROBE_PUSH_REF:-}" ]]; then
  must W-PUSH "$ARKS" push origin "HEAD:${ARKS_PROBE_PUSH_REF}"
  must W-PUSH-VERIFY sh -c "git ls-remote origin '${ARKS_PROBE_PUSH_REF}' | grep -q \"\$(git rev-parse HEAD)\""
fi

{
  printf '{\n  "probe": "cloud-dropin-host-probe",\n  "advisory": "host",\n'
  printf '  "arks_version": "%s",\n' "$("$ARKS" --version 2>&1 | head -1)"
  printf '  "git_version": "%s",\n' "$(git --version)"
  printf '  "must_failures": %d,\n  "rows": [\n' "$fails"
  for i in "${!rows[@]}"; do
    sep=","; [[ $i -eq $((${#rows[@]} - 1)) ]] && sep=""
    printf '    %s%s\n' "${rows[$i]}" "$sep"
  done
  printf '  ]\n}\n'
} >"$OUT/receipt.json"
cat "$OUT/receipt.json"
[[ $fails -eq 0 ]]
