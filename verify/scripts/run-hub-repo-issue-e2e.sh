#!/usr/bin/env bash
# C8 repo + issue rows — local-forge end-to-end (sot:root/arks-hub-repo-issue-depth).
# Entry: ARKS_HUB_IW_REPO_ISSUE=1 verify/scripts/run-hub-gh-arks-matrix-e2e.sh
#    or: verify/scripts/run-hub-repo-issue-e2e.sh [ARKS_BIN] [TIP]
# Uses only the arks binary (git HTTP + forge API under `arks serve`); MUST NOT spawn git/gh.
# Host runs are advisory; Claimed needs a Docker clean-room run (rule 28 / 60).
# ARKS_HUB_EVIDENCE_HOST=1 keeps host outputs under .spec/evidence/<ts>-hub-repo-issue/.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

ARKS_BIN="${1:-${ARKS_BIN:-./target/release/arks}}"
TIP="${2:-unknown}"
case "$ARKS_BIN" in /*) ;; *) ARKS_BIN="$ROOT/${ARKS_BIN#./}" ;; esac
[[ -x "$ARKS_BIN" ]] || { echo "FAIL: arks binary missing: $ARKS_BIN" >&2; exit 1; }
command -v python3 >/dev/null || { echo "FAIL: python3 missing" >&2; exit 1; }

free_port() {
  python3 - <<'PY'
import socket
s = socket.socket()
s.bind(("127.0.0.1", 0))
print(s.getsockname()[1])
s.close()
PY
}

wait_port() {
  python3 - "$1" <<'PY'
import socket, sys, time
port = int(sys.argv[1])
for _ in range(200):
    try:
        with socket.create_connection(("127.0.0.1", port), timeout=.2):
            raise SystemExit(0)
    except OSError:
        time.sleep(.05)
raise SystemExit(f"listener on {port} not ready")
PY
}

WORK="$(mktemp -d /tmp/arks-hub-repo-issue.XXXXXX)"
SERVER_PID=""
cleanup() {
  if [[ -n "$SERVER_PID" ]] && kill -0 "$SERVER_PID" 2>/dev/null; then
    kill "$SERVER_PID" 2>/dev/null || true
    wait "$SERVER_PID" 2>/dev/null || true
  fi
  rm -rf "$WORK"
}
trap cleanup EXIT

OUTS="$WORK/out"
mkdir -p "$OUTS" "$WORK/home/.config/arks" "$WORK/data/git/local"
export HOME="$WORK/home"
export ARKS_HUB_TOKEN_FILE="$HOME/.config/arks/hub-token"
export ARKS_HUB_TOKEN="local-dev-token"
export ARKS_HUB_CLIENT_STATE="$HOME/.config/arks/hub-client.json"
export ARKS_HUB_REPO="local/hubri"
HTTP_PORT="$(free_port)"
FORGE_PORT="$(free_port)"
export ARKS_HUB_API="http://127.0.0.1:${FORGE_PORT}"
export ARKS_HUB_CLONE_BASE="http://127.0.0.1:${HTTP_PORT}"
HOSTED="$WORK/data/git/local/hubri.git"
"$ARKS_BIN" init --bare "$HOSTED" >/dev/null

"$ARKS_BIN" serve --no-ssh --listen "127.0.0.1:${HTTP_PORT}" \
  --forge-listen "127.0.0.1:${FORGE_PORT}" --forge-data "$WORK/data" \
  "$HOSTED" >"$WORK/server.out" 2>"$WORK/server.err" &
SERVER_PID=$!
wait_port "$HTTP_PORT"
wait_port "$FORGE_PORT"

OK=1
# run NAME CMD... — expect exit 0; output kept as $OUTS/NAME.out
run() {
  local name="$1"; shift
  local ec
  set +e
  "$@" >"$OUTS/$name.out" 2>&1
  ec=$?
  set -e
  if [[ "$ec" -ne 0 ]]; then
    echo "FAIL C8 $name exit=$ec" >&2
    cat "$OUTS/$name.out" >&2
    OK=0
  else
    echo "hub-repo-issue-e2e: $name ok"
  fi
}
# refuse NAME CMD... — expect non-zero exit (fail-closed honesty)
refuse() {
  local name="$1"; shift
  local ec
  set +e
  "$@" >"$OUTS/$name.out" 2>&1
  ec=$?
  set -e
  if [[ "$ec" -eq 0 ]]; then
    echo "FAIL C8 $name invented success" >&2
    OK=0
  else
    echo "hub-repo-issue-e2e: $name refused ok (exit=$ec)"
  fi
}
# expect NAME PATTERN — fixed-string grep in $OUTS/NAME.out
expect() {
  if ! grep -Fq -- "$2" "$OUTS/$1.out"; then
    echo "FAIL C8 shape $1: missing '$2'" >&2
    OK=0
  fi
}

A="$ARKS_BIN"
# Seed the hosted repository over smart HTTP with arks itself.
mkdir -p "$WORK/src"
(
  cd "$WORK/src"
  "$A" init . >/dev/null
  "$A" config user.name "C8 E2E"
  "$A" config user.email c8@example.invalid
  mkdir docs
  printf 'hello\n' >README.md
  printf 'doc\n' >docs/a.md
  "$A" add .
  "$A" commit -q -m init
  "$A" remote add origin "$ARKS_HUB_CLONE_BASE/local/hubri.git"
  "$A" push origin main
) >"$OUTS/seed.out" 2>&1 || { echo "FAIL C8 seed" >&2; cat "$OUTS/seed.out" >&2; exit 1; }

run auth-login "$A" hub auth login
run repo-create "$A" hub repo create hubri --description C8
run repo-view "$A" hub repo view --repo local/hubri
expect repo-view '"clone_url"'

# read-dir / read-file (contents API)
run read-dir "$A" hub repo read-dir
expect read-dir 'docs/'
expect read-dir 'README.md'
run read-dir-json "$A" hub repo read-dir docs --json name,path,type,gitType,gitSHA
expect read-dir-json '"path": "docs/a.md"'
expect read-dir-json '"gitType": "blob"'
refuse read-dir-mode "$A" hub repo read-dir --json mode
run read-file "$A" hub repo read-file README.md
expect read-file 'hello'
run read-file-json "$A" hub repo read-file docs/a.md --json name,size,encoding
expect read-file-json '"encoding": "base64"'
run read-file-output "$A" hub repo read-file README.md --output "$WORK/readme.copy"
cmp -s "$WORK/readme.copy" "$WORK/src/README.md" || { echo "FAIL C8 read-file --output bytes" >&2; OK=0; }
refuse read-file-noclobber "$A" hub repo read-file README.md --output "$WORK/readme.copy"

# gitignore / license catalogs
run gitignore-list "$A" hub repo gitignore list
expect gitignore-list 'Rust'
run gitignore-view "$A" hub repo gitignore view Rust
expect gitignore-view '/target/'
refuse gitignore-view-missing "$A" hub repo gitignore view rust
run license-list "$A" hub repo license list
expect license-list 'mit'
run license-view "$A" hub repo license view MIT
expect license-view 'MIT License'

# autolinks
run autolink-create "$A" hub repo autolink create TICKET- 'https://example.invalid/T?q=<num>'
expect autolink-create 'Created repository autolink 1'
refuse autolink-create-bad "$A" hub repo autolink create BAD- 'https://example.invalid/no-placeholder'
run autolink-list "$A" hub repo autolink list
expect autolink-list 'TICKET-'
run autolink-view "$A" hub repo autolink view 1 --json id,keyPrefix,isAlphanumeric
expect autolink-view '"keyPrefix": "TICKET-"'
refuse autolink-delete-noyes "$A" hub repo autolink delete 1
run autolink-delete "$A" hub repo autolink delete 1 --yes
refuse autolink-view-gone "$A" hub repo autolink view 1

# deploy keys
printf 'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl c8@example\n' >"$WORK/deploy.pub"
run deploy-key-add "$A" hub repo deploy-key add "$WORK/deploy.pub" --title ci
run deploy-key-list "$A" hub repo deploy-key list --json id,title,readOnly
expect deploy-key-list '"readOnly": true'
expect deploy-key-list '"title": "ci"'
run deploy-key-delete "$A" hub repo deploy-key delete 1
run deploy-key-list-empty "$A" hub repo deploy-key list --json id
expect deploy-key-list-empty '[]'

# clone (API clone_url → in-process smart HTTP clone)
cd "$WORK"
run repo-clone "$A" hub repo clone local/hubri cl
cmp -s "$WORK/cl/README.md" "$WORK/src/README.md" || { echo "FAIL C8 clone content" >&2; OK=0; }
refuse repo-clone-nonempty "$A" hub repo clone local/hubri cl
refuse repo-clone-gitflag "$A" hub repo clone local/hubri cl2 -- --bare

# sync (local mode): fast-forward, dirty refuse, divergence refuse, --force
(cd "$WORK/src" && printf 'more\n' >>README.md && "$A" commit -q -a -m second && "$A" push origin main) >"$OUTS/src-second.out" 2>&1
cd "$WORK/cl"
run repo-sync-ff "$A" hub repo sync
expect repo-sync-ff 'Synced the "main" branch'
cmp -s "$WORK/cl/README.md" "$WORK/src/README.md" || { echo "FAIL C8 sync content" >&2; OK=0; }
cp README.md "$WORK/readme.keep"
printf 'dirty\n' >>README.md
refuse repo-sync-dirty "$A" hub repo sync
expect repo-sync-dirty 'local changes'
cp "$WORK/readme.keep" README.md
{ "$A" config user.name "C8 E2E" && "$A" config user.email c8@example.invalid; } >/dev/null 2>&1
printf 'local\n' >local.txt && "$A" add local.txt && "$A" commit -q -m local-only
(cd "$WORK/src" && printf 'third\n' >>README.md && "$A" commit -q -a -m third && "$A" push origin main) >"$OUTS/src-third.out" 2>&1
refuse repo-sync-diverged "$A" hub repo sync
expect repo-sync-diverged 'diverging changes'
run repo-sync-force "$A" hub repo sync --force
"$A" rev-parse HEAD >"$OUTS/cl-head.out"
(cd "$WORK/src" && "$A" rev-parse HEAD) >"$OUTS/src-head.out"
cmp -s "$OUTS/cl-head.out" "$OUTS/src-head.out" || { echo "FAIL C8 sync --force head" >&2; OK=0; }
[[ ! -e local.txt ]] || { echo "FAIL C8 sync --force left local.txt" >&2; OK=0; }
refuse repo-sync-remote "$A" hub repo sync local/hubri
cd "$WORK"

# issue artifacts
run issue-create "$A" hub issue create --title "Fix the bug" --body b
printf '# plan\n' >"$WORK/plan.md"
run artifact-create "$A" hub issue artifact create 1 "$WORK/plan.md" https://example.invalid/runbook
expect artifact-create 'generic'
expect artifact-create 'link'
run artifact-create-body "$A" hub issue artifact create 1 --body 'inline text' --name 'Notes (draft)'
expect artifact-create-body 'Notes draft'
refuse artifact-create-attach "$A" hub issue artifact create 1 "$WORK/plan.md" --attach x.png
run artifact-list "$A" hub issue artifact list 1 --json number,name,type
expect artifact-list '"type": "link"'
run artifact-list-links "$A" hub issue artifact list 1 --type link --json number
run artifact-edit "$A" hub issue artifact edit 1 1 --body '# plan v2'
run artifact-view-json "$A" hub issue artifact view 1 1 --json name,body,versions
expect artifact-view-json '# plan v2'
expect artifact-view-json '"version": 1'
run artifact-view-v1 "$A" hub issue artifact view 1 1 --version 1
expect artifact-view-v1 '# plan'
mkdir -p "$WORK/dl"
run artifact-download "$A" hub issue artifact download 1 --dir "$WORK/dl"
[[ -f "$WORK/dl/1-plan.md" && -f "$WORK/dl/2-example.invalid.url" ]] || { echo "FAIL C8 download names" >&2; ls "$WORK/dl" >&2; OK=0; }
grep -Fq 'URL=https://example.invalid/runbook' "$WORK/dl/2-example.invalid.url" || { echo "FAIL C8 .url shortcut" >&2; OK=0; }
refuse artifact-download-exists "$A" hub issue artifact download 1 1 --dir "$WORK/dl"
run artifact-download-stdout "$A" hub issue artifact download 1 1 --output -
expect artifact-download-stdout '# plan v2'
run artifact-restore "$A" hub issue artifact edit 1 1 --restore-version 1
refuse artifact-delete-noyes "$A" hub issue artifact delete 1 2
run artifact-delete "$A" hub issue artifact delete 1 2 --yes
run artifact-list-after "$A" hub issue artifact list 1 --json number
if grep -Fq '"number": 2' "$OUTS/artifact-list-after.out"; then echo "FAIL C8 artifact delete" >&2; OK=0; fi

# issue develop (GraphQL linked branches against the local forge)
run develop-create "$A" hub issue develop 1 --name feat-x --base main
expect develop-create 'feat-x'
run develop-default "$A" hub issue develop 1
expect develop-default '1-fix-the-bug'
refuse develop-dup "$A" hub issue develop 1 --name feat-x
run develop-list "$A" hub issue develop --list 1
expect develop-list 'feat-x'
expect develop-list '1-fix-the-bug'
refuse develop-worktree "$A" hub issue develop 1 --name feat-w --worktree "$WORK/wt"
cd "$WORK/cl"
run develop-checkout "$A" hub issue develop 1 --name feat-y --checkout
"$A" branch --show-current >"$OUTS/develop-branch.out" 2>&1
expect develop-branch 'feat-y'
cd "$WORK"

cp "$WORK/server.out" "$OUTS/server.out" 2>/dev/null || true

if [[ "$OK" -ne 1 ]]; then
  echo "hub-repo-issue-e2e: FAIL (see above)" >&2
  exit 1
fi
echo "hub-repo-issue-e2e: greens (local forge; not Claimed)"

venue=""
if [[ -f /.dockerenv ]]; then
  venue="docker"
elif [[ "${ARKS_HUB_EVIDENCE_HOST:-0}" == "1" ]]; then
  venue="host"
fi
if [[ -n "$venue" ]]; then
  TS="$(date -u +%Y%m%d%H%M%S)"
  OUT="$ROOT/.spec/evidence/${TS}-hub-repo-issue"
  mkdir -p "$OUT"
  # Scrub the per-run temp prefix from kept outputs.
  for f in "$OUTS"/*.out; do
    sed -e "s#${WORK}#\$WORK#g" "$f" >"$OUT/$(basename "$f")"
  done
  python3 - "$OUT/receipt.json" "$TIP" "$venue" <<'PY'
import json, sys, time
out, tip, venue = sys.argv[1:]
receipt = {
    "schema": "arks-hub-repo-issue/v1",
    "gate": "C8-HUB-REPO-ISSUE",
    "sot": "sot:root/arks-hub-repo-issue-depth",
    "venue": venue,
    "disposition": "host-advisory" if venue == "host" else "docker-clean-room-run",
    "tip": tip,
    "rows_exercised": [
        "repo clone", "repo sync", "repo read-dir", "repo read-file",
        "repo gitignore list", "repo gitignore view",
        "repo license list", "repo license view",
        "repo autolink create", "repo autolink delete", "repo autolink list", "repo autolink view",
        "repo deploy-key add", "repo deploy-key delete", "repo deploy-key list",
        "issue artifact create", "issue artifact delete", "issue artifact download",
        "issue artifact edit", "issue artifact list", "issue artifact view",
        "issue develop",
    ],
    "refusals_checked": [
        "read-dir --json mode", "repo clone -- --bare", "repo sync <destination>",
        "repo sync dirty", "repo sync diverged", "issue artifact create --attach",
        "issue develop --worktree", "delete without --yes",
    ],
    "claimed": [],
    "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
}
with open(out, "w", encoding="utf-8") as f:
    json.dump(receipt, f, indent=2)
    f.write("\n")
PY
  echo "hub-repo-issue-e2e: receipt=$OUT/receipt.json (venue=$venue)"
fi
