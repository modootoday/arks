#!/usr/bin/env bash
# G28 — live private GitHub journey coverage with arks (opt-in network).
# SoT: g28-live-private-arks-journeys; matrix: verify/matrix-g28.md
# Honesty: functional journeys ≠ scale/contention ≠ G16 timing compare.
# Uses arks-claimed argv only (no stock-Git-only -c / -u / ':' push refspecs).
set -euo pipefail

ROOT="${ARKS_G28_ROOT:-/workspace}"
if [[ -d /workspaces/gir/verify ]]; then
  ROOT="${ARKS_G28_ROOT:-/workspaces/gir}"
fi
cd "$ROOT"

mkdir -p verify/g28/out
RECEIPT="${ARKS_G28_RECEIPT_OUT:-$ROOT/verify/g28/out/receipt.json}"
RESULTS_JSON="${ARKS_G28_RESULTS_OUT:-$ROOT/verify/g28/out/journeys.json}"
REVISION="$(git -C "$ROOT" rev-parse HEAD 2>/dev/null || echo unknown)"
STOCK_GIT="$(git --version)"
IMAGE_TAG="${ARKS_JOURNEY_IMAGE_TAG:-arks-verify-m1:0.1.0}"
echo '[]' >"$RESULTS_JSON"

record() {
  local id="$1" status="$2" detail="$3"
  python3 - "$RESULTS_JSON" "$id" "$status" "$detail" <<'PY'
import json, sys
path, id_, status, detail = sys.argv[1:5]
with open(path, encoding="utf-8") as f:
    rows = json.load(f)
rows.append({"id": id_, "status": status, "detail": detail})
with open(path, "w", encoding="utf-8") as f:
    json.dump(rows, f, indent=2)
    f.write("\n")
print(f"G28: {id_} => {status} ({detail})")
PY
}

write_receipt() {
  local disposition="$1"
  local detail="$2"
  python3 - "$RECEIPT" "$RESULTS_JSON" "$disposition" "$detail" "$REVISION" "$STOCK_GIT" "$IMAGE_TAG" <<'PY'
import json, os, sys
out, results_path, disposition, detail, revision, stock_git, image_tag = sys.argv[1:8]
with open(results_path, encoding="utf-8") as f:
    journeys = json.load(f)
fail = sum(1 for j in journeys if j.get("status") == "FAIL")
passt = sum(1 for j in journeys if j.get("status") == "PASS")
na = sum(1 for j in journeys if j.get("status") == "N/A")
skip = sum(1 for j in journeys if j.get("status") == "SKIP")
receipt = {
    "schema": "arks-g28-live-private-journeys/v1",
    "gate": "G28",
    "name": "live-private-arks-journeys",
    "disposition": disposition,
    "detail": detail,
    "venue": "clean-room",
    "network": "opt-in-live",
    "revision": revision,
    "stock_git": stock_git,
    "image_tag": image_tag,
    "repo_url_public": os.environ.get("ARKS_G28_REPO_URL_PUBLIC", os.environ.get("ARKS_G28_REPO_URL", "")),
    "opt_in": {
        "ARKS_G28_LIVE": os.environ.get("ARKS_G28_LIVE", ""),
        "has_token": bool(os.environ.get("ARKS_G28_TOKEN") or os.environ.get("GITHUB_TOKEN")),
        "has_repo_url": bool(os.environ.get("ARKS_G28_REPO_URL")),
    },
    "counts": {"PASS": passt, "FAIL": fail, "N/A": na, "SKIP": skip, "total": len(journeys)},
    "journeys": journeys,
    "honesty": "functional journeys on live private forge; != G16 timing; != G26/G27 scale/contention; != Checks/merge-queue; unsigned seed (gpgsig unclaimed)",
    "adr_affinity": "adr:root/20261110151000",
    "sot": "sot:root/g28-live-private-arks-journeys",
}
with open(out, "w", encoding="utf-8") as f:
    json.dump(receipt, f, indent=2)
    f.write("\n")
print(f"G28: receipt written to {out} disposition={disposition} PASS={passt} FAIL={fail}")
PY
}

if ! command -v git >/dev/null; then
  echo "G28: stock git missing in image" >&2
  exit 1
fi
echo "G28: stock git ${STOCK_GIT}"

LIVE="${ARKS_G28_LIVE:-0}"
TOKEN="${ARKS_G28_TOKEN:-${GITHUB_TOKEN:-}}"
REPO_URL="${ARKS_G28_REPO_URL:-}"
export ARKS_G28_REPO_URL_PUBLIC="${ARKS_G28_REPO_URL_PUBLIC:-$REPO_URL}"

# Live receipt keeps N/A; checklist honesty closes via g28-offline-clones (evidence:root/20261109213400).
record "J-CLONE-LOCAL-01" "N/A" "offline — see g28-offline-clones / evidence:root/20261109213400"
record "J-CLONE-PATH-01" "N/A" "offline — see g28-offline-clones / evidence:root/20261109213400"

if [[ "${LIVE}" != "1" || -z "${TOKEN}" || -z "${REPO_URL}" ]]; then
  echo "G28: Refuse-closed (default) — missing opt-in live secrets"
  write_receipt "refuse-closed" "opt-in secrets absent; G16 offline + G7 remain default proofs"
  echo "G28: REFUSE-CLOSED OK"
  exit 0
fi

echo "G28: opt-in live private journey gate enabled (URL redacted)"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/cargo-target}"
cargo build -p arks --release
ARKS_BIN="$(find "${CARGO_TARGET_DIR}/release" target/release -name arks -type f 2>/dev/null | head -n1 || true)"
if [[ -z "${ARKS_BIN}" || ! -x "${ARKS_BIN}" ]]; then
  echo "G28: arks binary missing after build" >&2
  exit 1
fi
echo "G28: ARKS_BIN=${ARKS_BIN}"

AUTH_URL="$(python3 - "$REPO_URL" "$TOKEN" <<'PY'
import sys, urllib.parse
url, token = sys.argv[1], sys.argv[2]
u = urllib.parse.urlsplit(url)
netloc = f"x-access-token:{urllib.parse.quote(token, safe='')}@{u.hostname}"
if u.port:
    netloc += f":{u.port}"
print(urllib.parse.urlunsplit((u.scheme, netloc, u.path, u.query, u.fragment)))
PY
)"

OWNER_REPO="$(python3 - "$REPO_URL" <<'PY'
import sys, urllib.parse
u = urllib.parse.urlsplit(sys.argv[1])
parts = [p for p in u.path.strip('/').split('/') if p]
print(f"{parts[0]}/{parts[1].removesuffix('.git')}")
PY
)"
API="https://api.github.com/repos/${OWNER_REPO}"

WORK="$(mktemp -d /tmp/arks-g28.XXXXXX)"
trap 'rm -rf "$WORK"' EXIT

export GIT_AUTHOR_NAME="arks-g28"
export GIT_AUTHOR_EMAIL="arks-g28@example.com"
export GIT_COMMITTER_NAME="arks-g28"
export GIT_COMMITTER_EMAIL="arks-g28@example.com"
export GIT_AUTHOR_DATE="2026-11-10T00:00:00Z"
export GIT_COMMITTER_DATE="2026-11-10T00:00:00Z"

scrub_err() {
  tr '\n' ' ' </tmp/g28-step.err 2>/dev/null | head -c 400 | sed "s/${TOKEN}/***/g" || true
}

journey() {
  local id="$1"; shift
  if "$@" >/tmp/g28-step.out 2>/tmp/g28-step.err; then
    record "$id" "PASS" "ok"
    return 0
  fi
  record "$id" "FAIL" "$(scrub_err)"
  return 1
}

# --- clone ---
CLONE="${WORK}/clone"
if ! journey "J-REMOTE-CLONE-01" "${ARKS_BIN}" clone "${AUTH_URL}" "${CLONE}"; then
  write_receipt "fail" "clone failed; abort remaining live steps"
  exit 1
fi
cd "${CLONE}"
# arks remote set-url is unclaimed — clone already stored AUTH_URL in origin.
# Prefer stock git for --short (arks symbolic-ref does not claim --short).
DEFAULT_BRANCH="$(git -C "${CLONE}" symbolic-ref --short HEAD 2>/dev/null || echo main)"
[[ -n "${DEFAULT_BRANCH}" ]] || DEFAULT_BRANCH=main

sync_main() {
  cd "${CLONE}"
  "${ARKS_BIN}" switch "${DEFAULT_BRANCH}" >/dev/null 2>&1 || true
  "${ARKS_BIN}" fetch origin >/dev/null 2>&1 || true
  # Prefer hard reset via stock git inside clean-room when arks index dirty — advisory recovery only.
  git -C "${CLONE}" reset --hard "origin/${DEFAULT_BRANCH}" >/dev/null 2>&1 || \
    git -C "${CLONE}" reset --hard HEAD >/dev/null 2>&1 || true
  git -C "${CLONE}" clean -fd >/dev/null 2>&1 || true
  "${ARKS_BIN}" switch "${DEFAULT_BRANCH}" >/dev/null 2>&1 || true
}

# --- J-LOCAL-01 ---
journey "J-LOCAL-01" bash -c "
  set -e
  cd '${CLONE}'
  echo 'g28-local' > g28-local.txt
  '${ARKS_BIN}' add g28-local.txt
  '${ARKS_BIN}' commit -m 'g28 J-LOCAL-01 seed'
  '${ARKS_BIN}' status
  '${ARKS_BIN}' log -1 --oneline
  '${ARKS_BIN}' push origin '${DEFAULT_BRANCH}'
"

# --- J-BRANCH-01 ---
sync_main
journey "J-BRANCH-01" bash -c "
  set -e
  cd '${CLONE}'
  '${ARKS_BIN}' branch topic-g28
  '${ARKS_BIN}' checkout topic-g28
  echo topic > topic.txt
  '${ARKS_BIN}' add topic.txt
  '${ARKS_BIN}' commit -m 'g28 topic'
  '${ARKS_BIN}' switch '${DEFAULT_BRANCH}'
  '${ARKS_BIN}' switch topic-g28
  '${ARKS_BIN}' push origin topic-g28
"

# --- J-PUSH-FETCH-01 ---
journey "J-PUSH-FETCH-01" bash -c "
  set -e
  cd '${CLONE}'
  '${ARKS_BIN}' fetch origin
  tip=\"\$('${ARKS_BIN}' rev-parse origin/topic-g28)\"
  test -n \"\$tip\"
"

# --- J-MERGE-FF-01 ---
sync_main
journey "J-MERGE-FF-01" bash -c "
  set -e
  cd '${CLONE}'
  '${ARKS_BIN}' fetch origin
  '${ARKS_BIN}' switch '${DEFAULT_BRANCH}'
  '${ARKS_BIN}' merge --ff-only topic-g28 || '${ARKS_BIN}' merge --ff-only origin/topic-g28
  '${ARKS_BIN}' push origin '${DEFAULT_BRANCH}'
"

# --- J-PULL-01 ---
PULL_DIR="${WORK}/pull-clone"
journey "J-PULL-01" bash -c "
  set -e
  '${ARKS_BIN}' clone '${AUTH_URL}' '${PULL_DIR}'
  cd '${CLONE}'
  echo pull-bait > pull-bait.txt
  '${ARKS_BIN}' add pull-bait.txt
  '${ARKS_BIN}' commit -m 'g28 pull bait'
  '${ARKS_BIN}' push origin '${DEFAULT_BRANCH}'
  cd '${PULL_DIR}'
  '${ARKS_BIN}' pull origin
  test -f pull-bait.txt
"

# --- J-TAG-01 ---
sync_main
journey "J-TAG-01" bash -c "
  set -e
  cd '${CLONE}'
  '${ARKS_BIN}' tag -a -m 'g28 tag' g28-v1
  '${ARKS_BIN}' tag
  '${ARKS_BIN}' push origin refs/tags/g28-v1
"

# --- J-STASH-01 ---
sync_main
journey "J-STASH-01" bash -c "
  set -e
  cd '${CLONE}'
  echo dirty > stash-me.txt
  '${ARKS_BIN}' add stash-me.txt
  echo dirtier >> stash-me.txt
  '${ARKS_BIN}' stash push
  '${ARKS_BIN}' stash list
  '${ARKS_BIN}' stash pop
"

# --- J-DIFF-01 ---
sync_main
journey "J-DIFF-01" bash -c "
  set -e
  cd '${CLONE}'
  echo diff-a > diff-a.txt
  '${ARKS_BIN}' add diff-a.txt
  '${ARKS_BIN}' commit -m 'g28 diff base'
  echo diff-b >> diff-a.txt
  '${ARKS_BIN}' diff >/dev/null
  '${ARKS_BIN}' add diff-a.txt
  '${ARKS_BIN}' diff --cached >/dev/null
  '${ARKS_BIN}' commit -m 'g28 diff commit'
  '${ARKS_BIN}' push origin '${DEFAULT_BRANCH}'
"

# --- J-RM-MV-01 ---
sync_main
journey "J-RM-MV-01" bash -c "
  set -e
  cd '${CLONE}'
  echo mvsrc > mv-src.txt
  echo rmme > rm-me.txt
  '${ARKS_BIN}' add mv-src.txt rm-me.txt
  '${ARKS_BIN}' commit -m 'g28 rm-mv seed'
  '${ARKS_BIN}' mv mv-src.txt mv-dst.txt
  '${ARKS_BIN}' rm rm-me.txt
  '${ARKS_BIN}' commit -m 'g28 rm-mv'
  '${ARKS_BIN}' push origin '${DEFAULT_BRANCH}'
"

# --- J-CHERRY-01 ---
sync_main
journey "J-CHERRY-01" bash -c "
  set -e
  cd '${CLONE}'
  '${ARKS_BIN}' switch -c cherry-src
  echo cherry > cherry.txt
  '${ARKS_BIN}' add cherry.txt
  '${ARKS_BIN}' commit -m 'g28 cherry commit'
  CHERRY_SHA=\"\$('${ARKS_BIN}' rev-parse HEAD)\"
  '${ARKS_BIN}' switch '${DEFAULT_BRANCH}'
  '${ARKS_BIN}' cherry-pick \"\$CHERRY_SHA\"
  '${ARKS_BIN}' push origin '${DEFAULT_BRANCH}'
"

# --- J-REBASE-01 ---
sync_main
journey "J-REBASE-01" bash -c "
  set -e
  cd '${CLONE}'
  '${ARKS_BIN}' switch -c rebase-topic
  echo rebase-a > rebase-a.txt
  '${ARKS_BIN}' add rebase-a.txt
  '${ARKS_BIN}' commit -m 'g28 rebase topic'
  '${ARKS_BIN}' switch '${DEFAULT_BRANCH}'
  echo rebase-main > rebase-main.txt
  '${ARKS_BIN}' add rebase-main.txt
  '${ARKS_BIN}' commit -m 'g28 rebase main advance'
  '${ARKS_BIN}' switch rebase-topic
  '${ARKS_BIN}' rebase '${DEFAULT_BRANCH}'
  '${ARKS_BIN}' switch '${DEFAULT_BRANCH}'
  '${ARKS_BIN}' merge --ff-only rebase-topic
  '${ARKS_BIN}' push origin '${DEFAULT_BRANCH}'
"

# --- J-REVERT-01 ---
sync_main
journey "J-REVERT-01" bash -c "
  set -e
  cd '${CLONE}'
  echo revert-me > revert-me.txt
  '${ARKS_BIN}' add revert-me.txt
  '${ARKS_BIN}' commit -m 'g28 to revert'
  '${ARKS_BIN}' revert HEAD
  '${ARKS_BIN}' push origin '${DEFAULT_BRANCH}'
"

# --- J-REFLOG-01 ---
journey "J-REFLOG-01" bash -c "
  set -e
  cd '${CLONE}'
  '${ARKS_BIN}' reflog
  '${ARKS_BIN}' reflog show
"

# --- J-BUNDLE-01 ---
journey "J-BUNDLE-01" bash -c "
  set -e
  cd '${CLONE}'
  '${ARKS_BIN}' bundle create '${WORK}/g28.bundle' HEAD
  '${ARKS_BIN}' bundle verify '${WORK}/g28.bundle'
"

# --- J-ARCHIVE-01 ---
journey "J-ARCHIVE-01" bash -c "
  set -e
  cd '${CLONE}'
  '${ARKS_BIN}' archive --format=tar -o '${WORK}/g28.tar' HEAD
  test -s '${WORK}/g28.tar'
"

# --- J-REPACK-01 ---
journey "J-REPACK-01" bash -c "
  set -e
  cd '${CLONE}'
  '${ARKS_BIN}' repack -ad
"

# --- J-WORKTREE-01 ---
sync_main
journey "J-WORKTREE-01" bash -c "
  set -e
  cd '${CLONE}'
  WT='${WORK}/wt-g28'
  '${ARKS_BIN}' worktree add -b wt-branch \"\$WT\"
  '${ARKS_BIN}' worktree list
  '${ARKS_BIN}' worktree remove \"\$WT\"
"

# --- J-SUBMODULE-01 (local-path peer; HTTPS submodule add Claimed via G7 loopback FW-SUBMOD-HTTPS) ---
SUB_URL="${ARKS_G28_SUBMODULE_URL:-}"
if [[ -n "${SUB_URL}" ]]; then
  SUB_AUTH="$(python3 - "$SUB_URL" "$TOKEN" <<'PY'
import sys, urllib.parse
url, token = sys.argv[1], sys.argv[2]
u = urllib.parse.urlsplit(url)
netloc = f"x-access-token:{urllib.parse.quote(token, safe='')}@{u.hostname}"
print(urllib.parse.urlunsplit((u.scheme, netloc, u.path, u.query, u.fragment)))
PY
)"
  sync_main
  journey "J-SUBMODULE-01" bash -c "
    set -e
    '${ARKS_BIN}' clone '${SUB_AUTH}' '${WORK}/sub-src'
    cd '${CLONE}'
    '${ARKS_BIN}' submodule add '${WORK}/sub-src' vendor/g28-sub
    '${ARKS_BIN}' submodule status
    '${ARKS_BIN}' commit -m 'g28 submodule'
    '${ARKS_BIN}' push origin '${DEFAULT_BRANCH}'
  "
else
  record "J-SUBMODULE-01" "SKIP" "ARKS_G28_SUBMODULE_URL unset"
fi

# --- J-PR-GH-01 ---
sync_main
journey "J-PR-GH-01" bash -c "
  set -e
  cd '${CLONE}'
  '${ARKS_BIN}' switch -c pr-g28
  echo pr-body > pr-g28.txt
  '${ARKS_BIN}' add pr-g28.txt
  '${ARKS_BIN}' commit -m 'g28 pr branch'
  '${ARKS_BIN}' push origin pr-g28
  python3 - '${API}' '${TOKEN}' '${DEFAULT_BRANCH}' <<'PY'
import json, sys, urllib.request
api, token, base = sys.argv[1:4]
req = urllib.request.Request(
    f'{api}/pulls',
    data=json.dumps({
        'title': 'g28 live private journey PR',
        'head': 'pr-g28',
        'base': base,
        'body': 'Ephemeral G28 journey verification PR (safe to close/delete with repo).',
    }).encode(),
    headers={
        'Authorization': f'Bearer {token}',
        'Accept': 'application/vnd.github+json',
        'X-GitHub-Api-Version': '2022-11-28',
        'User-Agent': 'arks-g28',
        'Content-Type': 'application/json',
    },
    method='POST',
)
with urllib.request.urlopen(req, timeout=60) as resp:
    body = json.load(resp)
print(body.get('html_url', 'pr-created'))
PY
"

# --- J-ALIAS-01 ---
journey "J-ALIAS-01" bash -c "
  set -e
  SHIM='${WORK}/shim'
  mkdir -p \"\$SHIM\"
  printf '#!/bin/bash\nexec \"%s\" \"\$@\"\n' '${ARKS_BIN}' > \"\$SHIM/git\"
  chmod +x \"\$SHIM/git\"
  ALIAS_DIR='${WORK}/alias'
  mkdir -p \"\$ALIAS_DIR\"
  cd \"\$ALIAS_DIR\"
  PATH=\"\$SHIM:\$PATH\" git init
  echo alias > a.txt
  PATH=\"\$SHIM:\$PATH\" git add a.txt
  PATH=\"\$SHIM:\$PATH\" git commit -m 'alias'
  PATH=\"\$SHIM:\$PATH\" git log -1 --oneline
"

FAIL_COUNT="$(python3 - "$RESULTS_JSON" <<'PY'
import json,sys
rows=json.load(open(sys.argv[1],encoding='utf-8'))
print(sum(1 for r in rows if r['status']=='FAIL'))
PY
)"

if [[ "${FAIL_COUNT}" -gt 0 ]]; then
  write_receipt "fail" "one or more journeys FAILED count=${FAIL_COUNT}"
  echo "G28: FAIL (${FAIL_COUNT})"
  exit 1
fi

write_receipt "closed-implement" "all non-N/A live journeys PASS or SKIP"
echo "G28: PASS (live private journeys)"
