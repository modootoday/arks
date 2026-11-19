#!/usr/bin/env bash
# DoD pillar 3 — gh vs arks hub matrix e2e + local multi-agent PR scenarios.
# Entry is `arks hub` (single release binary). MUST NOT spawn `gh` or `git`.
# Sibling matrix SoT: .spec/sot/arks-hub-gh-command-matrix.sot.md (+ CSV beside it).
# IW-HUB-FLAT Claimed subset: sot:root/arks-verify-docker-e2e-claim-wave (ARKS_HUB_IW_FLAT=1).
# IW-HUB-FULL expand + Missing→0: sot:root/arks-remaining-incompletes-wave (ARKS_HUB_IW_FULL=1).
# IW-HUB-MUTATE subset: sot:root/arks-honesty-leftovers-wave (ARKS_HUB_IW_MUTATE=1).
# IW-HUB-ASSETS / CHECKOUT / UX / STUBS: sot:root/arks-in-scope-leftovers-wave.
# ARKS_HUB_IW_ACTIONS=1: platform-only OUT refuse + in-scope `later` refuse check (adr:root/20261119230000).
# ARKS_HUB_IW_FORGE_GROUPS=1: arks-owned gist/discussion/project/ssh-key/gpg-key/
#   repo autolink/repo deploy-key leaves against `arks serve` (host run = Partial; adr:root/20261119230000).
# B3 client remote ≠ flat-leaf Claimed.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

SOT="${ARKS_HUB_MATRIX_SOT:-.spec/sot/arks-hub-gh-command-matrix.sot.md}"
DOCKER_SOT="${ARKS_DOCKER_E2E_CLAIM_SOT:-.spec/sot/arks-verify-docker-e2e-claim-wave.sot.md}"
INCOMPLETES_SOT="${ARKS_INCOMPLETES_SOT:-.spec/sot/arks-remaining-incompletes-wave.sot.md}"
HONESTY_SOT="${ARKS_HONESTY_LEFTOVERS_SOT:-.spec/sot/arks-honesty-leftovers-wave.sot.md}"
SCOPE_SOT="${ARKS_IN_SCOPE_LEFTOVERS_SOT:-.spec/sot/arks-in-scope-leftovers-wave.sot.md}"
# Living e2e CSV (preferred) or design CSV from sibling SoT.
if [[ -z "${ARKS_HUB_MATRIX_CSV:-}" ]]; then
  if [[ -f verify/matrices/hub-gh-arks-e2e-matrix.csv ]]; then
    CSV=verify/matrices/hub-gh-arks-e2e-matrix.csv
  else
    CSV=.spec/sot/arks-hub-gh-command-matrix.csv
  fi
else
  CSV="$ARKS_HUB_MATRIX_CSV"
fi
SCENARIO="${ARKS_HUB_LOCAL_SCENARIO:-.spec/sot/arks-hub-local-multi-agent-pr.sot.md}"
UMBRELLA="${ARKS_SYMMETRY_UMBRELLA_SOT:-.spec/sot/verify-cleanroom-symmetry-matrix.sot.md}"
TIP="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"

echo "hub-gh-arks-matrix-e2e: tip=${TIP}"

ARKS_BIN="${ARKS_BIN:-}"
if [[ -z "$ARKS_BIN" ]]; then
  if [[ -x "${CARGO_TARGET_DIR:-}/release/arks" ]]; then
    ARKS_BIN="${CARGO_TARGET_DIR}/release/arks"
  elif [[ -x ./.cargo-target/release/arks ]]; then
    ARKS_BIN=./.cargo-target/release/arks
  elif [[ -x ./target/release/arks ]]; then
    ARKS_BIN=./target/release/arks
  elif [[ -x ./.cargo-target/debug/arks ]]; then
    ARKS_BIN=./.cargo-target/debug/arks
  elif [[ -x ./target/debug/arks ]]; then
    ARKS_BIN=./target/debug/arks
  elif [[ -n "${CARGO_TARGET_DIR:-}" && -x "${CARGO_TARGET_DIR}/debug/arks" ]]; then
    ARKS_BIN="${CARGO_TARGET_DIR}/debug/arks"
  else
    export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"
    cargo build -p arks --release
    ARKS_BIN="${CARGO_TARGET_DIR:-target}/release/arks"
    [[ -x "$ARKS_BIN" ]] || ARKS_BIN=./.cargo-target/release/arks
    [[ -x "$ARKS_BIN" ]] || ARKS_BIN=./target/release/arks
  fi
fi

help_out="$("$ARKS_BIN" hub --help 2>&1 || true)"
if ! echo "$help_out" | grep -qi 'arks hub'; then
  echo "FAIL: arks hub --help missing expected banner"
  exit 1
fi
if echo "$help_out" | grep -Eqi 'spawn(s|ing)? gh|exec(ute)? gh|Command::new\("gh"\)'; then
  echo "FAIL: hub help implies gh subprocess"
  exit 1
fi
echo "hub-gh-arks-matrix-e2e: arks hub --help ok (no gh spawn language)"

if command -v arks-hub >/dev/null 2>&1; then
  echo "note: arks-hub on PATH is non-normative; DoD entry is arks hub"
fi

if [[ ! -f "$SOT" ]]; then
  echo "hub-gh-arks-matrix-e2e: SKIP full matrix — missing $SOT"
  exit 0
fi

for g in auth pr issue release repo; do
  if ! echo "$help_out" | grep -qw "$g"; then
    echo "FAIL: arks hub --help missing group $g (SoT discoverability)"
    exit 1
  fi
done

if [[ ! -f "$CSV" ]]; then
  echo "hub-gh-arks-matrix-e2e: SKIP row e2e — waiting CSV $CSV"
  echo "hub-gh-arks-matrix-e2e: smoke-only PASS (help + SoT present); row e2e not Claimed"
  exit 0
fi

header="$(head -n1 "$CSV" | sed 's/^\xEF\xBB\xBF//' || true)"
# Quoted CSV fields: strip " before compare (BOM-tolerant header above).
if echo "$header" | grep -q 'status_now'; then
  # Living e2e: gh_command,arks_hub_target,status_now,phase,notes
  v1=$(awk -F, 'NR>1 {gsub(/"/,"",$3); gsub(/"/,"",$4); if($4=="v1") c++} END{print c+0}' "$CSV")
  missing=$(awk -F, 'NR>1 {gsub(/"/,"",$3); gsub(/"/,"",$4); if($4=="v1" && $3=="Missing") c++} END{print c+0}' "$CSV")
  refuse=$(awk -F, 'NR>1 {gsub(/"/,"",$3); if($3=="Refuse/N/A") c++} END{print c+0}' "$CSV")
  echo "hub-gh-arks-matrix-e2e: csv=$CSV v1=$v1 Missing(v1)=$missing Refuse=$refuse"
else
  # Design CSV: gh_pin,gh_command,arks_hub_target,phase,notes
  v1=$(awk -F, 'NR>1 {gsub(/"/,"",$4); if($4=="v1") c++} END{print c+0}' "$CSV")
  missing=0
  refuse=$(awk -F, 'NR>1 {gsub(/"/,"",$3); gsub(/"/,"",$4); if($3=="OUT" || $4=="out-of-scope") c++} END{print c+0}' "$CSV")
  echo "hub-gh-arks-matrix-e2e: csv=$CSV v1=$v1 OUT=$refuse (design table)"
fi

if [[ "$v1" -eq 0 ]]; then
  echo "FAIL: hub matrix CSV has no v1 rows"
  exit 1
fi

# --- shared forge bootstrap helper for in-scope leftovers ---
hub_scope_boot() {
  local work_var="$1"
  export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"
  cargo build -p arks --release
  ARKS_BIN="${CARGO_TARGET_DIR:-target}/release/arks"
  [[ -x "$ARKS_BIN" ]] || ARKS_BIN=./.cargo-target/release/arks
  [[ -x "$ARKS_BIN" ]] || ARKS_BIN=./target/release/arks
  [[ -x "$ARKS_BIN" ]] || {
    echo "FAIL: arks binary missing after rebuild" >&2
    return 1
  }
  WORK="$(mktemp -d /tmp/arks-hub-scope.XXXXXX)"
  eval "$work_var=\"\$WORK\""
  FORGE_PORT="$(python3 - <<'PY'
import socket
s = socket.socket()
s.bind(("127.0.0.1", 0))
print(s.getsockname()[1])
s.close()
PY
)"
  FORGE_LISTEN="127.0.0.1:${FORGE_PORT}"
  mkdir -p "$WORK/hub-data" "$WORK/home/.config/arks"
  export HOME="$WORK/home"
  export ARKS_HUB_TOKEN_FILE="$WORK/home/.config/arks/hub-token"
  export ARKS_HUB_TOKEN="local-dev-token"
  export ARKS_HUB_API="http://${FORGE_LISTEN}"
  export ARKS_HUB_REPO="local/hubscope"
  export ARKS_HUB_CLIENT_STATE="$WORK/home/.config/arks/hub-client.json"
  "$ARKS_BIN" serve --no-ssh --no-http \
    --forge-listen "$FORGE_LISTEN" \
    --forge-data "$WORK/hub-data" \
    . >"$WORK/server.out" 2>"$WORK/server.err" &
  SERVER_PID=$!
  python3 - "$FORGE_PORT" <<'PY'
import socket, sys, time
port = int(sys.argv[1])
for _ in range(200):
    try:
        with socket.create_connection(("127.0.0.1", port), timeout=.2):
            raise SystemExit(0)
    except OSError:
        time.sleep(.05)
raise SystemExit("hub-scope forge listener not ready")
PY
}

hub_scope_run() {
  local name="$1"
  shift
  local out ec
  set +e
  out="$("$@" 2>&1)"
  ec=$?
  set -e
  printf '%s\n' "$out" >"$WORK/${name}.out"
  if [[ "$ec" -ne 0 ]]; then
    echo "FAIL HF-SCOPE $name exit=$ec" >&2
    echo "$out" >&2
    SCOPE_OK=0
    return 0
  fi
  echo "hub-gh-arks-matrix-e2e: HF-SCOPE $name ok"
}

# --- Platform-only OUT refuse + in-scope later refuse (ARKS_HUB_IW_ACTIONS=1) ---
# GitHub-platform-only groups (Actions run/workflow/cache/secret/variable,
# attestations, Codespaces) and leaves (release verify*) are out of scope for
# the arks forge. Rulesets (`ruleset` / `rs`), `repo gitignore|license` and
# `pr checks` are in scope but `later` ([[adr:root/20261119230000]]). Each must
# refuse with exit 2 and make no forge call: ARKS_HUB_API points at a closed
# port so a stray network call fails instead of greening. Later rows must say
# "later — in scope". This is refuse honesty, not a Claimed feature.
if [[ "${ARKS_HUB_IW_ACTIONS:-0}" == "1" ]]; then
  echo "hub-gh-arks-matrix-e2e: platform-only OUT refuse path (adr:root/20261119230000)"
  OUT_WORK="$(mktemp -d /tmp/arks-hub-out.XXXXXX)"
  trap 'rm -rf "${OUT_WORK:-}"' EXIT
  export HOME="$OUT_WORK/home"
  mkdir -p "$HOME/.config/arks"
  export ARKS_HUB_TOKEN_FILE="$HOME/.config/arks/hub-token"
  export ARKS_HUB_API="http://127.0.0.1:9"
  export ARKS_HUB_REPO="local/hubscope"
  OUT_OK=1
  while IFS= read -r leaf; do
    [[ -z "$leaf" ]] && continue
    set +e
    # shellcheck disable=SC2086
    "$ARKS_BIN" hub $leaf >"$OUT_WORK/out.txt" 2>&1
    oec=$?
    set -e
    if [[ "$oec" -ne 2 ]]; then
      echo "FAIL platform-only OUT: hub $leaf exit=$oec (want 2)" >&2
      cat "$OUT_WORK/out.txt" >&2 || true
      OUT_OK=0
    else
      echo "hub-gh-arks-matrix-e2e: OUT refuse ok (hub $leaf exit=2)"
    fi
  done <<'LEAVES'
run list
run view 1
workflow list
workflow run 1
cache list
secret list
variable list
attestation verify
codespace list
release verify v1.0.0
release verify-asset v1.0.0 a.bin
LEAVES
  while IFS= read -r leaf; do
    [[ -z "$leaf" ]] && continue
    set +e
    # shellcheck disable=SC2086
    "$ARKS_BIN" hub $leaf >"$OUT_WORK/out.txt" 2>&1
    oec=$?
    set -e
    if [[ "$oec" -ne 2 ]] || ! grep -Fq 'later — in scope, not yet implemented' "$OUT_WORK/out.txt"; then
      echo "FAIL in-scope later refuse: hub $leaf exit=$oec (want 2 + later text)" >&2
      cat "$OUT_WORK/out.txt" >&2 || true
      OUT_OK=0
    else
      echo "hub-gh-arks-matrix-e2e: later refuse ok (hub $leaf exit=2)"
    fi
  done <<'LEAVES'
ruleset list
rs list
ruleset view 1
ruleset check main
repo gitignore list
repo gitignore view Rust
repo license list
repo license view MIT
pr checks 1
LEAVES
  if [[ "$OUT_OK" -ne 1 ]]; then
    if [[ "${ARKS_DOD_STRICT:-0}" == "1" ]]; then
      echo "hub-gh-arks-matrix-e2e: FAIL STRICT platform-only OUT refuse" >&2
      exit 1
    fi
    echo "hub-gh-arks-matrix-e2e: platform-only OUT refuse Partial/failed"
    exit 0
  fi
  echo "hub-gh-arks-matrix-e2e: platform-only OUT refuse greens"
  exit 0
fi

# --- arks-owned forge groups (ARKS_HUB_IW_FORGE_GROUPS=1) ---
# gist / discussion / project / ssh-key / gpg-key / repo autolink / repo deploy-key
# against a local `arks serve` forge ([[adr:root/20261119230000]] item 7). Key inputs
# are public halves of ephemeral fixtures (the same ones crate unit tests pin against
# GnuPG / OpenSSH fingerprints). Host run = Partial (host-advisory); a Docker run only
# writes a receipt, it does not promote rows by itself.
if [[ "${ARKS_HUB_IW_FORGE_GROUPS:-0}" == "1" ]]; then
  echo "hub-gh-arks-matrix-e2e: arks-owned forge groups path (adr:root/20261119230000)"
  command -v python3 >/dev/null || { echo "FAIL: python3 missing" >&2; exit 1; }
  SERVER_PID=""
  cleanup_scope() {
    if [[ -n "${SERVER_PID:-}" ]] && kill -0 "$SERVER_PID" 2>/dev/null; then
      kill "$SERVER_PID" 2>/dev/null || true
      wait "$SERVER_PID" 2>/dev/null || true
    fi
    rm -rf "${WORK:-}"
  }
  trap cleanup_scope EXIT
  hub_scope_boot WORK
  SCOPE_OK=1
  export ARKS_HUB_REPO="local/hubscope"
  printf 'alpha\n' >"$WORK/a.txt"
  printf 'alpha v2\n' >"$WORK/a2.txt"
  printf '# notes\n' >"$WORK/notes.md"
  cat >"$WORK/id.pub" <<'KEY'
ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOeNU8Aa/AgRlkpxyZRX7JNqG5QCAhOXZdT1/GhUi3TM fixture@example.invalid
KEY
  cat >"$WORK/pub.asc" <<'KEY'
-----BEGIN PGP PUBLIC KEY BLOCK-----

mDMEav9wixYJKwYBBAHaRw8BAQdAQjLeQBeoT4aDP0hMcp8V0S5KliR4Wo7OTC0l
qsx/yCK0JUZpeHR1cmUgS2V5IDxmaXh0dXJlQGV4YW1wbGUuaW52YWxpZD6IkwQT
FgoAOxYhBI7bByOhOpVqZC07Mi8wt6QweLGoBQJq/3CLAhsDBQsJCAcCAiICBhUK
CQgLAgQWAgMBAh4HAheAAAoJEC8wt6QweLGoQmcA/j1iFqpGmluCZTSL06ANxSXg
tb3A038eqruIHmAhLWu5AQDLo/2RiEm7W4TOdcPttVtdoGWKtQK53SBDHVsTh/EZ
AA==
=SSXi
-----END PGP PUBLIC KEY BLOCK-----
KEY
  hub_scope_run auth-login "$ARKS_BIN" hub auth login
  hub_scope_run repo-create "$ARKS_BIN" hub repo create hubscope --description "forge groups"
  hub_scope_run issue-create "$ARKS_BIN" hub issue create --title tracked --body b
  # gist
  hub_scope_run gist-create "$ARKS_BIN" gist create "$WORK/a.txt" "$WORK/notes.md" -d demo --public
  GIST_ID="$(sed -n 's#.*/gists/##p' "$WORK/gist-create.out" | tail -n1)"
  hub_scope_run gist-list "$ARKS_BIN" gist list
  hub_scope_run gist-view "$ARKS_BIN" gist view "$GIST_ID" -f a.txt
  hub_scope_run gist-edit "$ARKS_BIN" gist edit "$GIST_ID" "$WORK/a2.txt" -f a.txt
  hub_scope_run gist-rename "$ARKS_BIN" gist rename "$GIST_ID" notes.md readme.md
  hub_scope_run gist-clone "$ARKS_BIN" gist clone "$GIST_ID" "$WORK/gist-dir"
  # discussion
  hub_scope_run disc-create "$ARKS_BIN" discussion create -t plan -b body -c Ideas
  hub_scope_run disc-list "$ARKS_BIN" discussion list --json number,title,category
  hub_scope_run disc-edit "$ARKS_BIN" discussion edit 1 -t plan-v2
  hub_scope_run disc-comment "$ARKS_BIN" discussion comment 1 -b hello
  hub_scope_run disc-view "$ARKS_BIN" discussion view 1 --comments
  # project
  hub_scope_run proj-create "$ARKS_BIN" project create --title Roadmap
  hub_scope_run proj-edit "$ARKS_BIN" project edit 1 -d short --visibility PUBLIC
  hub_scope_run proj-field "$ARKS_BIN" project field-create 1 --name Status --data-type SINGLE_SELECT --single-select-options Todo,Done --format json
  hub_scope_run proj-field2 "$ARKS_BIN" project field-create 1 --name Note --data-type TEXT
  hub_scope_run proj-fields "$ARKS_BIN" project field-list 1
  hub_scope_run proj-item-add "$ARKS_BIN" project item-add 1 --url "local/hubscope#1"
  hub_scope_run proj-item-create "$ARKS_BIN" project item-create 1 --title draft --format json
  ITEM_ID="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["id"])' "$WORK/proj-item-create.out")"
  hub_scope_run proj-item-edit "$ARKS_BIN" project item-edit --id "$ITEM_ID" --project-id 1 --field-id 2 --text hi
  hub_scope_run proj-items "$ARKS_BIN" project item-list 1 --format json
  hub_scope_run proj-item-archive "$ARKS_BIN" project item-archive 1 --id "$ITEM_ID"
  hub_scope_run proj-copy "$ARKS_BIN" project copy 1 --title Copy --drafts
  hub_scope_run proj-item-delete "$ARKS_BIN" project item-delete 1 --id "$ITEM_ID"
  hub_scope_run proj-field-delete "$ARKS_BIN" project field-delete --id 2
  hub_scope_run proj-link "$ARKS_BIN" project link 1 -R local/hubscope
  hub_scope_run proj-unlink "$ARKS_BIN" project unlink 1 -R local/hubscope
  hub_scope_run proj-template "$ARKS_BIN" project mark-template 1
  hub_scope_run proj-close "$ARKS_BIN" project close 1
  hub_scope_run proj-list "$ARKS_BIN" project list --closed --format json
  hub_scope_run proj-view "$ARKS_BIN" project view 1
  hub_scope_run proj-delete "$ARKS_BIN" project delete 2
  # keys
  hub_scope_run ssh-add "$ARKS_BIN" ssh-key add "$WORK/id.pub" -t laptop
  hub_scope_run ssh-list "$ARKS_BIN" ssh-key list
  hub_scope_run ssh-delete "$ARKS_BIN" ssh-key delete 1 --yes
  hub_scope_run gpg-add "$ARKS_BIN" gpg-key add "$WORK/pub.asc"
  hub_scope_run gpg-list "$ARKS_BIN" gpg-key list
  hub_scope_run gpg-delete "$ARKS_BIN" gpg-key delete 2F30B7A43078B1A8 --yes
  hub_scope_run dk-add "$ARKS_BIN" hub repo deploy-key add "$WORK/id.pub" -t ci -w
  hub_scope_run dk-list "$ARKS_BIN" hub repo deploy-key list --json id,title,readOnly
  hub_scope_run dk-delete "$ARKS_BIN" hub repo deploy-key delete 1
  # autolink
  hub_scope_run al-create "$ARKS_BIN" hub repo autolink create TICKET- 'https://tracker.example/t/<num>'
  hub_scope_run al-list "$ARKS_BIN" hub repo autolink list
  hub_scope_run al-view "$ARKS_BIN" hub repo autolink view 1 --json keyPrefix,urlTemplate
  hub_scope_run al-delete "$ARKS_BIN" hub repo autolink delete 1 --yes
  # Invalid input must fail closed (non-zero), never invent success.
  for bad in "ssh-key add $WORK/a.txt" "gpg-key add $WORK/a.txt" \
             "hub repo autolink create X- https://no-placeholder" "gist edit $GIST_ID"; do
    set +e
    # shellcheck disable=SC2086
    "$ARKS_BIN" $bad >"$WORK/bad.out" 2>&1
    bec=$?
    set -e
    if [[ "$bec" -eq 0 ]]; then
      echo "FAIL forge-groups invalid input greened: $bad" >&2
      SCOPE_OK=0
    else
      echo "hub-gh-arks-matrix-e2e: forge-groups refuse ok ($bad exit=$bec)"
    fi
  done
  hub_scope_run gist-delete "$ARKS_BIN" gist delete "$GIST_ID" --yes
  if [[ "$SCOPE_OK" -eq 1 ]]; then
    if ! grep -Fq $'demo\t2 files\tpublic' "$WORK/gist-list.out" \
      || ! grep -Fxq 'alpha' "$WORK/gist-view.out" \
      || ! grep -Fxq 'alpha v2' "$WORK/gist-dir/a.txt" \
      || [[ ! -f "$WORK/gist-dir/readme.md" ]] \
      || ! grep -Fq 'recipe=' "$WORK/gist-clone.out" \
      || ! grep -Fq '"Ideas"' "$WORK/disc-list.out" \
      || ! grep -Fq 'hello' "$WORK/disc-view.out" \
      || ! grep -Fq $'Status\tSINGLE_SELECT' "$WORK/proj-fields.out" \
      || ! grep -Fq '"note": "hi"' "$WORK/proj-items.out" \
      || ! grep -Fq '"template": true' "$WORK/proj-list.out" \
      || ! grep -Fq 'SHA256:MO2X5WSxNRrSc36ZeuLqwBzZc44TWFE2llj7/mJ1s2w' "$WORK/ssh-add.out" \
      || ! grep -Fq $'laptop\t1\tssh-ed25519' "$WORK/ssh-list.out" \
      || ! grep -Fq $'fixture@example.invalid\t2F30B7A43078B1A8\t8EDB0723A13A956A642D3B322F30B7A43078B1A8' "$WORK/gpg-list.out" \
      || ! grep -Fq '"readOnly": false' "$WORK/dk-list.out" \
      || ! grep -Fq $'TICKET-\thttps://tracker.example/t/<num>\ttrue' "$WORK/al-list.out"; then
      echo "FAIL forge-groups output shapes" >&2
      SCOPE_OK=0
    fi
  fi
  if [[ "$SCOPE_OK" -ne 1 ]]; then
    cat "$WORK/server.err" >&2 || true
    if [[ "${ARKS_DOD_STRICT:-0}" == "1" ]]; then
      echo "hub-gh-arks-matrix-e2e: FAIL STRICT forge groups" >&2
      exit 1
    fi
    echo "hub-gh-arks-matrix-e2e: forge groups Partial/failed"
    exit 0
  fi
  echo "hub-gh-arks-matrix-e2e: forge groups greens (host run = Partial unless Docker receipt)"
  if [[ -f /.dockerenv ]]; then
    TS="$(date -u +%Y%m%d%H%M%S)"
    OUT=".spec/evidence/${TS}-hub-forge-groups"
    mkdir -p "$OUT"
    cp "$WORK"/*.out "$OUT/" 2>/dev/null || true
    python3 - "$OUT/receipt.json" "$TIP" "${ARKS_JOURNEY_IMAGE_TAG:-arks-verify-m1:0.1.0}" <<'PY'
import json, sys, time
out, tip, image = sys.argv[1:]
receipt = {
    "schema": "arks-hub-forge-groups/v1",
    "gate": "IW-HUB-FORGE-GROUPS",
    "disposition": "product-path-run",
    "venue": "docker-dod-hub-matrix",
    "tip": tip,
    "image_tag": image,
    "groups": ["gist", "discussion", "project", "ssh-key", "gpg-key",
               "repo autolink", "repo deploy-key"],
    "note": "Receipt only; row promotion needs an owning SoT update.",
    "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
}
with open(out, "w", encoding="utf-8") as f:
    json.dump(receipt, f, indent=2)
    f.write("\n")
PY
    echo "hub-gh-arks-matrix-e2e: receipt=$OUT/receipt.json"
  fi
  exit 0
fi

# --- IW-HUB-ASSETS (ARKS_HUB_IW_ASSETS=1) ---
if [[ "${ARKS_HUB_IW_ASSETS:-0}" == "1" ]]; then
  echo "hub-gh-arks-matrix-e2e: IW-HUB-ASSETS path (sot:root/arks-in-scope-leftovers-wave)"
  [[ -f "$SCOPE_SOT" ]] || { echo "FAIL: missing $SCOPE_SOT" >&2; exit 1; }
  command -v python3 >/dev/null || { echo "FAIL: python3 missing" >&2; exit 1; }
  SERVER_PID=""
  cleanup_scope() {
    if [[ -n "${SERVER_PID:-}" ]] && kill -0 "$SERVER_PID" 2>/dev/null; then
      kill "$SERVER_PID" 2>/dev/null || true
      wait "$SERVER_PID" 2>/dev/null || true
    fi
    rm -rf "${WORK:-}"
  }
  trap cleanup_scope EXIT
  hub_scope_boot WORK
  SCOPE_OK=1
  echo 'asset-bytes-v1' >"$WORK/payload.bin"
  hub_scope_run auth-login "$ARKS_BIN" hub auth login
  hub_scope_run repo-create "$ARKS_BIN" hub repo create hubscope --description "IW-HUB-ASSETS"
  hub_scope_run release-create "$ARKS_BIN" hub release create --repo local/hubscope --tag v1.0.0 --title r --notes n
  hub_scope_run release-upload "$ARKS_BIN" hub release upload --repo local/hubscope --tag v1.0.0 --file "$WORK/payload.bin" --name payload.bin
  hub_scope_run release-download "$ARKS_BIN" hub release download --repo local/hubscope --asset-id 1 --output "$WORK/got.bin"
  hub_scope_run release-edit "$ARKS_BIN" hub release edit --repo local/hubscope --tag v1.0.0 --title r-ed --notes n2
  hub_scope_run release-delete-asset "$ARKS_BIN" hub release delete-asset --repo local/hubscope --asset-id 1
  hub_scope_run release-create2 "$ARKS_BIN" hub release create --repo local/hubscope --tag v1.0.1 --title r2 --notes n
  hub_scope_run release-delete "$ARKS_BIN" hub release delete --repo local/hubscope --tag v1.0.1
  if [[ "$SCOPE_OK" -eq 1 ]]; then
    if ! cmp -s "$WORK/payload.bin" "$WORK/got.bin"; then
      echo "FAIL HF-SCOPE asset round-trip" >&2
      SCOPE_OK=0
    fi
    if ! grep -Fq 'r-ed' "$WORK/release-edit.out"; then
      echo "FAIL HF-SCOPE release edit" >&2
      SCOPE_OK=0
    fi
  fi
  if [[ "$SCOPE_OK" -ne 1 ]]; then
    cat "$WORK/server.err" >&2 || true
    if [[ "${ARKS_DOD_STRICT:-0}" == "1" ]]; then
      echo "hub-gh-arks-matrix-e2e: FAIL STRICT IW-HUB-ASSETS" >&2
      exit 1
    fi
    echo "hub-gh-arks-matrix-e2e: IW-HUB-ASSETS Partial/failed"
    exit 0
  fi
  echo "hub-gh-arks-matrix-e2e: IW-HUB-ASSETS greens"
  if [[ -f /.dockerenv ]]; then
    TS="$(date -u +%Y%m%d%H%M%S)"
    OUT=".spec/evidence/${TS}-hub-release-assets"
    mkdir -p "$OUT"
    cp "$WORK"/*.out "$OUT/" 2>/dev/null || true
    python3 - "$OUT/receipt.json" "$TIP" "${ARKS_JOURNEY_IMAGE_TAG:-arks-verify-m1:0.1.0}" <<'PY'
import json, sys, time
out, tip, image = sys.argv[1:]
receipt = {
    "schema": "arks-hub-release-assets/v1",
    "gate": "IW-HUB-ASSETS",
    "disposition": "verified-product-path",
    "venue": "docker-dod-hub-matrix",
    "tip": tip,
    "image_tag": image,
    "claimed": ["release upload", "release download", "release delete-asset", "release edit", "release delete"],
    "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
}
with open(out, "w", encoding="utf-8") as f:
    json.dump(receipt, f, indent=2)
    f.write("\n")
PY
    echo "hub-gh-arks-matrix-e2e: receipt=$OUT/receipt.json"
  fi
  exit 0
fi

# --- IW-HUB-CHECKOUT (ARKS_HUB_IW_CHECKOUT=1) ---
if [[ "${ARKS_HUB_IW_CHECKOUT:-0}" == "1" ]]; then
  echo "hub-gh-arks-matrix-e2e: IW-HUB-CHECKOUT path (sot:root/arks-in-scope-leftovers-wave)"
  [[ -f "$SCOPE_SOT" ]] || { echo "FAIL: missing $SCOPE_SOT" >&2; exit 1; }
  command -v python3 >/dev/null || { echo "FAIL: python3 missing" >&2; exit 1; }
  SERVER_PID=""
  cleanup_scope() {
    if [[ -n "${SERVER_PID:-}" ]] && kill -0 "$SERVER_PID" 2>/dev/null; then
      kill "$SERVER_PID" 2>/dev/null || true
      wait "$SERVER_PID" 2>/dev/null || true
    fi
    rm -rf "${WORK:-}"
  }
  trap cleanup_scope EXIT
  hub_scope_boot WORK
  SCOPE_OK=1
  hub_scope_run auth-login "$ARKS_BIN" hub auth login
  hub_scope_run repo-create "$ARKS_BIN" hub repo create hubscope --description "IW-HUB-CHECKOUT"
  hub_scope_run pr-create "$ARKS_BIN" hub pr create --repo local/hubscope --title co-pr --body b --head feature --base main
  hub_scope_run pr-checkout "$ARKS_BIN" hub pr checkout 1 --repo local/hubscope
  hub_scope_run co-alias "$ARKS_BIN" hub co 1 --repo local/hubscope
  if [[ "$SCOPE_OK" -eq 1 ]]; then
    if ! grep -Fq 'head_ref=feature' "$WORK/pr-checkout.out" \
      || ! grep -Fq 'recipe=arks' "$WORK/pr-checkout.out" \
      || ! grep -Fq 'head_ref=feature' "$WORK/co-alias.out"; then
      echo "FAIL HF-SCOPE checkout resolve/print" >&2
      SCOPE_OK=0
    fi
  fi
  if [[ "$SCOPE_OK" -ne 1 ]]; then
    cat "$WORK/server.err" >&2 || true
    if [[ "${ARKS_DOD_STRICT:-0}" == "1" ]]; then
      echo "hub-gh-arks-matrix-e2e: FAIL STRICT IW-HUB-CHECKOUT" >&2
      exit 1
    fi
    echo "hub-gh-arks-matrix-e2e: IW-HUB-CHECKOUT Partial/failed"
    exit 0
  fi
  echo "hub-gh-arks-matrix-e2e: IW-HUB-CHECKOUT greens"
  if [[ -f /.dockerenv ]]; then
    TS="$(date -u +%Y%m%d%H%M%S)"
    OUT=".spec/evidence/${TS}-hub-pr-checkout"
    mkdir -p "$OUT"
    cp "$WORK"/*.out "$OUT/" 2>/dev/null || true
    python3 - "$OUT/receipt.json" "$TIP" "${ARKS_JOURNEY_IMAGE_TAG:-arks-verify-m1:0.1.0}" <<'PY'
import json, sys, time
out, tip, image = sys.argv[1:]
receipt = {
    "schema": "arks-hub-pr-checkout/v1",
    "gate": "IW-HUB-CHECKOUT",
    "disposition": "verified-product-path",
    "venue": "docker-dod-hub-matrix",
    "tip": tip,
    "image_tag": image,
    "claimed": ["pr checkout", "co"],
    "note": "Resolve/print only; never spawn git/gh.",
    "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
}
with open(out, "w", encoding="utf-8") as f:
    json.dump(receipt, f, indent=2)
    f.write("\n")
PY
    echo "hub-gh-arks-matrix-e2e: receipt=$OUT/receipt.json"
  fi
  exit 0
fi

# --- IW-HUB-UX (ARKS_HUB_IW_UX=1) ---
if [[ "${ARKS_HUB_IW_UX:-0}" == "1" ]]; then
  echo "hub-gh-arks-matrix-e2e: IW-HUB-UX path (sot:root/arks-in-scope-leftovers-wave)"
  [[ -f "$SCOPE_SOT" ]] || { echo "FAIL: missing $SCOPE_SOT" >&2; exit 1; }
  command -v python3 >/dev/null || { echo "FAIL: python3 missing" >&2; exit 1; }
  SERVER_PID=""
  cleanup_scope() {
    if [[ -n "${SERVER_PID:-}" ]] && kill -0 "$SERVER_PID" 2>/dev/null; then
      kill "$SERVER_PID" 2>/dev/null || true
      wait "$SERVER_PID" 2>/dev/null || true
    fi
    rm -rf "${WORK:-}"
  }
  trap cleanup_scope EXIT
  hub_scope_boot WORK
  SCOPE_OK=1
  printf 'demo=arks hub status\n' >"$WORK/aliases.txt"
  hub_scope_run config-set "$ARKS_BIN" hub config set editor nano
  hub_scope_run config-get "$ARKS_BIN" hub config get edito
  hub_scope_run config-list "$ARKS_BIN" hub config list
  hub_scope_run alias-set "$ARKS_BIN" hub alias set demo 'arks hub status'
  hub_scope_run alias-list "$ARKS_BIN" hub alias list
  hub_scope_run alias-import "$ARKS_BIN" hub alias import "$WORK/aliases.txt"
  hub_scope_run alias-delete "$ARKS_BIN" hub alias delete demo
  hub_scope_run completion-bash "$ARKS_BIN" hub completion bash
  hub_scope_run config-clear "$ARKS_BIN" hub config clear-cache
  if [[ "$SCOPE_OK" -eq 1 ]]; then
    if ! grep -Fxq 'nano' "$WORK/config-get.out" \
      || ! grep -Fq 'complete -W' "$WORK/completion-bash.out"; then
      echo "FAIL HF-SCOPE UX shapes" >&2
      SCOPE_OK=0
    fi
  fi
  if [[ "$SCOPE_OK" -ne 1 ]]; then
    if [[ "${ARKS_DOD_STRICT:-0}" == "1" ]]; then
      echo "hub-gh-arks-matrix-e2e: FAIL STRICT IW-HUB-UX" >&2
      exit 1
    fi
    echo "hub-gh-arks-matrix-e2e: IW-HUB-UX Partial/failed"
    exit 0
  fi
  echo "hub-gh-arks-matrix-e2e: IW-HUB-UX greens"
  if [[ -f /.dockerenv ]]; then
    TS="$(date -u +%Y%m%d%H%M%S)"
    OUT=".spec/evidence/${TS}-hub-ux-config-alias"
    mkdir -p "$OUT"
    cp "$WORK"/*.out "$OUT/" 2>/dev/null || true
    python3 - "$OUT/receipt.json" "$TIP" "${ARKS_JOURNEY_IMAGE_TAG:-arks-verify-m1:0.1.0}" <<'PY'
import json, sys, time
out, tip, image = sys.argv[1:]
receipt = {
    "schema": "arks-hub-ux/v1",
    "gate": "IW-HUB-UX",
    "disposition": "verified-product-path",
    "venue": "docker-dod-hub-matrix",
    "tip": tip,
    "image_tag": image,
    "claimed": [
        "config get", "config set", "config list", "config clear-cache",
        "alias set", "alias list", "alias delete", "alias import", "completion",
    ],
    "note": "Client state file + completion emit; no shell profile installer mutation.",
    "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
}
with open(out, "w", encoding="utf-8") as f:
    json.dump(receipt, f, indent=2)
    f.write("\n")
PY
    echo "hub-gh-arks-matrix-e2e: receipt=$OUT/receipt.json"
  fi
  exit 0
fi

# --- IW-HUB-STUBS expand + exhaust (ARKS_HUB_IW_STUBS=1 or ARKS_HUB_IW_STUBS2=1) ---
# sot:root/arks-in-scope-leftovers-wave + research:root/20261111133600 + 20261111135200
if [[ "${ARKS_HUB_IW_STUBS:-0}" == "1" || "${ARKS_HUB_IW_STUBS2:-0}" == "1" ]]; then
  echo "hub-gh-arks-matrix-e2e: IW-HUB-STUBS(+exhaust) path (sot:root/arks-in-scope-leftovers-wave)"
  [[ -f "$SCOPE_SOT" ]] || { echo "FAIL: missing $SCOPE_SOT" >&2; exit 1; }
  command -v python3 >/dev/null || { echo "FAIL: python3 missing" >&2; exit 1; }
  SERVER_PID=""
  cleanup_scope() {
    if [[ -n "${SERVER_PID:-}" ]] && kill -0 "$SERVER_PID" 2>/dev/null; then
      kill "$SERVER_PID" 2>/dev/null || true
      wait "$SERVER_PID" 2>/dev/null || true
    fi
    rm -rf "${WORK:-}"
  }
  trap cleanup_scope EXIT
  hub_scope_boot WORK
  SCOPE_OK=1
  export ARKS_HUB_REPO="local/hubscope"
  hub_scope_run auth-login "$ARKS_BIN" hub auth login
  hub_scope_run repo-create "$ARKS_BIN" hub repo create hubscope --description "IW-HUB-STUBS"
  hub_scope_run repo-create2 "$ARKS_BIN" hub repo create hubdest --description "transfer-dest"
  hub_scope_run repo-edit "$ARKS_BIN" hub repo edit --repo local/hubscope --description "stubs-ed"
  hub_scope_run repo-set-default "$ARKS_BIN" hub repo set-default local/hubscope
  hub_scope_run issue-create "$ARKS_BIN" hub issue create --title stub-issue --body b
  hub_scope_run issue-status "$ARKS_BIN" hub issue status 1
  hub_scope_run issue-lock "$ARKS_BIN" hub issue lock 1
  hub_scope_run issue-unlock "$ARKS_BIN" hub issue unlock 1
  hub_scope_run issue-pin "$ARKS_BIN" hub issue pin 1
  hub_scope_run issue-unpin "$ARKS_BIN" hub issue unpin 1
  hub_scope_run issue-transfer "$ARKS_BIN" hub issue transfer 1 --to local/hubdest
  # After transfer, next create on hubscope gets a fresh number; delete that leaf.
  hub_scope_run issue-create2 "$ARKS_BIN" hub issue create --repo local/hubscope --title del-me --body b
  # Parse number from create2 JSON body (hub_http_print dumps JSON).
  DEL_NUM="$(python3 -c 'import re; t=open("'"$WORK"'/issue-create2.out",encoding="utf-8").read(); m=re.search(r"\"number\"\s*:\s*(\d+)", t); print(m.group(1) if m else "2")')"
  hub_scope_run issue-delete "$ARKS_BIN" hub issue delete "$DEL_NUM" --repo local/hubscope
  hub_scope_run pr-create-draft "$ARKS_BIN" hub pr create --title stub-pr --body pb --head f --base main --draft
  hub_scope_run pr-status-draft "$ARKS_BIN" hub pr status 1
  hub_scope_run pr-ready "$ARKS_BIN" hub pr ready 1
  hub_scope_run pr-status-ready "$ARKS_BIN" hub pr status 1
  hub_scope_run pr-lock "$ARKS_BIN" hub pr lock 1
  hub_scope_run pr-unlock "$ARKS_BIN" hub pr unlock 1
  hub_scope_run pr-review "$ARKS_BIN" hub pr review 1 --body lgtm --event APPROVE
  hub_scope_run label-create "$ARKS_BIN" hub label create --repo local/hubscope --name bug --color ff0000
  hub_scope_run label-clone "$ARKS_BIN" hub label clone --repo local/hubdest --from local/hubscope
  hub_scope_run repo-archive "$ARKS_BIN" hub repo archive --repo local/hubscope
  hub_scope_run repo-unarchive "$ARKS_BIN" hub repo unarchive --repo local/hubscope
  hub_scope_run repo-rename "$ARKS_BIN" hub repo rename --repo local/hubscope hubrenamed
  export ARKS_HUB_REPO="local/hubrenamed"
  hub_scope_run api-get "$ARKS_BIN" hub api --method GET /repos/local/hubrenamed
  hub_scope_run status-agg "$ARKS_BIN" hub status
  set +e
  "$ARKS_BIN" hub pr >"$WORK/group-help-pr.out" 2>&1
  gh_ec=$?
  set -e
  if [[ "$gh_ec" -ne 0 ]]; then
    echo "FAIL HF-SCOPE group-help-pr exit=$gh_ec" >&2
    cat "$WORK/group-help-pr.out" >&2 || true
    SCOPE_OK=0
  else
    echo "hub-gh-arks-matrix-e2e: HF-SCOPE group-help-pr ok"
  fi
  hub_scope_run repo-delete "$ARKS_BIN" hub repo delete --repo local/hubdest
  # Terminal Partial honesty: git/CI/OAuth must stay exit 2
  for term in "pr diff 1" "pr checks 1" "pr update-branch 1" "pr revert 1" "repo fork" "auth refresh"; do
    set +e
    # shellcheck disable=SC2086
    "$ARKS_BIN" hub $term >"$WORK/term-${term// /_}.out" 2>&1
    tec=$?
    set -e
    if [[ "$tec" -eq 0 ]]; then
      echo "FAIL HF-SCOPE terminal invent PASS: hub $term" >&2
      SCOPE_OK=0
    else
      echo "hub-gh-arks-matrix-e2e: HF-SCOPE terminal Partial ok ($term exit=$tec)"
    fi
  done
  if [[ "$SCOPE_OK" -eq 1 ]]; then
    if ! grep -Fq 'description' "$WORK/repo-edit.out" \
      || ! grep -Fq 'default_repo=local/hubscope' "$WORK/repo-set-default.out" \
      || ! grep -Fq "deleted=${DEL_NUM}" "$WORK/issue-delete.out" \
      || ! grep -Fq 'draft=true' "$WORK/pr-status-draft.out" \
      || ! grep -Fq 'ready=true' "$WORK/pr-ready.out" \
      || ! grep -Fq 'locked=true' "$WORK/pr-lock.out" \
      || ! grep -Fq 'locked=false' "$WORK/pr-unlock.out" \
      || ! grep -Fq 'state=APPROVED' "$WORK/pr-review.out" \
      || ! grep -Fq 'locked=true' "$WORK/issue-lock.out" \
      || ! grep -Fq 'locked=false' "$WORK/issue-unlock.out" \
      || ! grep -Fq 'pinned=true' "$WORK/issue-pin.out" \
      || ! grep -Fq 'pinned=false' "$WORK/issue-unpin.out" \
      || ! grep -Fq 'repository=local/hubdest' "$WORK/issue-transfer.out" \
      || ! grep -Fq 'cloned=' "$WORK/label-clone.out" \
      || ! grep -Fq 'archived=true' "$WORK/repo-archive.out" \
      || ! grep -Fq 'archived=false' "$WORK/repo-unarchive.out" \
      || ! grep -Fq 'name=hubrenamed' "$WORK/repo-rename.out" \
      || ! grep -Fq 'deleted=local/hubdest' "$WORK/repo-delete.out" \
      || ! grep -Fq 'http_status=200' "$WORK/api-get.out" \
      || ! grep -Fq 'issues_open=' "$WORK/status-agg.out"; then
      echo "FAIL HF-SCOPE STUBS exhaust shapes" >&2
      SCOPE_OK=0
    fi
  fi
  if [[ "$SCOPE_OK" -ne 1 ]]; then
    if [[ "${ARKS_DOD_STRICT:-0}" == "1" ]]; then
      echo "hub-gh-arks-matrix-e2e: FAIL STRICT IW-HUB-STUBS" >&2
      exit 1
    fi
    echo "hub-gh-arks-matrix-e2e: IW-HUB-STUBS Partial/failed"
    exit 0
  fi
  echo "hub-gh-arks-matrix-e2e: IW-HUB-STUBS greens"
  if [[ -f /.dockerenv ]]; then
    TS="$(date -u +%Y%m%d%H%M%S)"
    OUT=".spec/evidence/${TS}-hub-stubs-exhaust"
    mkdir -p "$OUT"
    cp "$WORK"/*.out "$OUT/" 2>/dev/null || true
    python3 - "$OUT/receipt.json" "$TIP" "${ARKS_JOURNEY_IMAGE_TAG:-arks-verify-m1:0.1.0}" <<'PY'
import json, sys, time
out, tip, image = sys.argv[1:]
receipt = {
    "schema": "arks-hub-stubs/v2",
    "gate": "IW-HUB-STUBS",
    "disposition": "verified-product-path",
    "venue": "docker-dod-hub-matrix",
    "tip": tip,
    "image_tag": image,
    "claimed": [
        "pr status", "pr ready", "pr lock", "pr unlock", "pr review",
        "issue status", "issue delete", "issue lock", "issue unlock", "issue pin", "issue unpin", "issue transfer",
        "repo edit", "repo set-default", "repo archive", "repo unarchive", "repo rename", "repo delete",
        "label clone", "api", "status",
        "group-help discoverability",
    ],
    "terminal_partial": [
        "pr diff", "pr checks", "pr update-branch", "pr revert",
        "repo fork", "auth refresh",
    ],
    "note": "Metadata Claimed + terminal Partial cites research:root/20261111135200.",
    "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
}
with open(out, "w", encoding="utf-8") as f:
    json.dump(receipt, f, indent=2)
    f.write("\n")
PY
    echo "hub-gh-arks-matrix-e2e: receipt=$OUT/receipt.json"
  fi
  exit 0
fi

# --- IW-HUB-MUTATE subset (ARKS_HUB_IW_MUTATE=1) ---
# sot:root/arks-honesty-leftovers-wave — mutate/view Claimed list only with receipt.
if [[ "${ARKS_HUB_IW_MUTATE:-0}" == "1" ]]; then
  echo "hub-gh-arks-matrix-e2e: IW-HUB-MUTATE path (sot:root/arks-honesty-leftovers-wave)"
  if [[ ! -f "$HONESTY_SOT" ]]; then
    echo "FAIL: missing honesty leftovers SoT $HONESTY_SOT" >&2
    exit 1
  fi
  command -v python3 >/dev/null || {
    echo "FAIL: python3 missing for IW-HUB-MUTATE" >&2
    exit 1
  }
  # Always rebuild so mutate leaves are present (no stale binary Claimed).
  export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"
  cargo build -p arks --release
  ARKS_BIN="${CARGO_TARGET_DIR:-target}/release/arks"
  [[ -x "$ARKS_BIN" ]] || ARKS_BIN=./.cargo-target/release/arks
  [[ -x "$ARKS_BIN" ]] || ARKS_BIN=./target/release/arks
  [[ -x "$ARKS_BIN" ]] || {
    echo "FAIL: arks binary missing after rebuild" >&2
    exit 1
  }

  WORK="$(mktemp -d /tmp/arks-hub-mutate.XXXXXX)"
  SERVER_PID=""
  cleanup_mut() {
    if [[ -n "${SERVER_PID:-}" ]] && kill -0 "$SERVER_PID" 2>/dev/null; then
      kill "$SERVER_PID" 2>/dev/null || true
      wait "$SERVER_PID" 2>/dev/null || true
    fi
    rm -rf "$WORK"
  }
  trap cleanup_mut EXIT

  FORGE_PORT="$(python3 - <<'PY'
import socket
s = socket.socket()
s.bind(("127.0.0.1", 0))
print(s.getsockname()[1])
s.close()
PY
)"
  FORGE_LISTEN="127.0.0.1:${FORGE_PORT}"
  mkdir -p "$WORK/hub-data" "$WORK/home/.config/arks"
  export HOME="$WORK/home"
  export ARKS_HUB_TOKEN_FILE="$WORK/home/.config/arks/hub-token"
  export ARKS_HUB_TOKEN="local-dev-token"
  export ARKS_HUB_API="http://${FORGE_LISTEN}"
  export ARKS_HUB_REPO="local/hubmut"

  "$ARKS_BIN" serve --no-ssh --no-http \
    --forge-listen "$FORGE_LISTEN" \
    --forge-data "$WORK/hub-data" \
    . >"$WORK/server.out" 2>"$WORK/server.err" &
  SERVER_PID=$!

  python3 - "$FORGE_PORT" <<'PY'
import socket, sys, time
port = int(sys.argv[1])
for _ in range(200):
    try:
        with socket.create_connection(("127.0.0.1", port), timeout=.2):
            raise SystemExit(0)
    except OSError:
        time.sleep(.05)
raise SystemExit("hub-mutate forge listener not ready")
PY

  MUT_OK=1
  run_mut() {
    local name="$1"
    shift
    local out ec
    set +e
    out="$("$@" 2>&1)"
    ec=$?
    set -e
    printf '%s\n' "$out" >"$WORK/${name}.out"
    if [[ "$ec" -ne 0 ]]; then
      echo "FAIL HF-MUT $name exit=$ec" >&2
      echo "$out" >&2
      MUT_OK=0
      return 0
    fi
    echo "hub-gh-arks-matrix-e2e: HF-MUT $name ok"
  }

  run_mut auth-login "$ARKS_BIN" hub auth login
  run_mut repo-create "$ARKS_BIN" hub repo create hubmut --description "IW-HUB-MUTATE"
  run_mut issue-create "$ARKS_BIN" hub issue create --repo local/hubmut --title "mut-issue" --body "body0"
  run_mut issue-edit "$ARKS_BIN" hub issue edit 1 --repo local/hubmut --title "mut-issue-ed" --body "body1"
  run_mut issue-comment "$ARKS_BIN" hub issue comment 1 --repo local/hubmut --body "ic1"
  run_mut issue-close "$ARKS_BIN" hub issue close 1 --repo local/hubmut
  run_mut issue-view-closed "$ARKS_BIN" hub issue view 1 --repo local/hubmut
  run_mut issue-reopen "$ARKS_BIN" hub issue reopen 1 --repo local/hubmut
  run_mut pr-create "$ARKS_BIN" hub pr create --repo local/hubmut --title "mut-pr" --body "pb0" --head f --base main
  run_mut pr-edit "$ARKS_BIN" hub pr edit 1 --repo local/hubmut --title "mut-pr-ed" --body "pb1"
  run_mut pr-comment "$ARKS_BIN" hub pr comment 1 --repo local/hubmut --body "pc1"
  run_mut pr-close "$ARKS_BIN" hub pr close 1 --repo local/hubmut
  run_mut pr-reopen "$ARKS_BIN" hub pr reopen 1 --repo local/hubmut
  run_mut pr-merge "$ARKS_BIN" hub pr merge 1 --repo local/hubmut
  run_mut release-create "$ARKS_BIN" hub release create --repo local/hubmut --tag v0.1.0 --title "r" --notes "n"
  run_mut release-view "$ARKS_BIN" hub release view --repo local/hubmut --tag v0.1.0
  run_mut label-create "$ARKS_BIN" hub label create --repo local/hubmut --name bug --color ff0000 --description "b"
  run_mut label-edit "$ARKS_BIN" hub label edit --repo local/hubmut --name bug --color 00ff00 --description "fixed"
  run_mut label-list-before-del "$ARKS_BIN" hub label list --repo local/hubmut
  run_mut label-delete "$ARKS_BIN" hub label delete --repo local/hubmut --name bug
  run_mut label-list-after-del "$ARKS_BIN" hub label list --repo local/hubmut

  if [[ "$MUT_OK" -eq 1 ]]; then
    if ! grep -Fq '"state":"closed"' "$WORK/issue-view-closed.out" \
      || ! grep -Fq 'mut-issue-ed' "$WORK/issue-edit.out" \
      || ! grep -Fq 'ic1' "$WORK/issue-comment.out" \
      || ! grep -Fq 'pc1' "$WORK/pr-comment.out" \
      || ! grep -Fq '"merged":true' "$WORK/pr-merge.out" \
      || ! grep -Fq 'v0.1.0' "$WORK/release-view.out" \
      || ! grep -Fq '00ff00' "$WORK/label-edit.out" \
      || ! grep -Fq 'bug' "$WORK/label-list-before-del.out" \
      || grep -Fq 'bug' "$WORK/label-list-after-del.out"; then
      echo "FAIL HF-MUT peer-faithful mutate shapes" >&2
      MUT_OK=0
    else
      echo "hub-gh-arks-matrix-e2e: HF-MUT peer-faithful shapes ok"
    fi
  fi

  if [[ "$MUT_OK" -ne 1 ]]; then
    cat "$WORK/server.err" >&2 || true
    if [[ "${ARKS_DOD_STRICT:-0}" == "1" ]]; then
      echo "hub-gh-arks-matrix-e2e: FAIL STRICT IW-HUB-MUTATE (no invent PASS)" >&2
      exit 1
    fi
    echo "hub-gh-arks-matrix-e2e: IW-HUB-MUTATE Partial/failed"
    exit 0
  fi

  echo "hub-gh-arks-matrix-e2e: IW-HUB-MUTATE greens"

  if [[ -f /.dockerenv ]]; then
    TS="$(date -u +%Y%m%d%H%M%S)"
    OUT=".spec/evidence/${TS}-hub-mutate-subset"
    mkdir -p "$OUT"
    cp "$WORK/server.err" "$OUT/server.err" 2>/dev/null || true
    cp "$WORK"/*.out "$OUT/" 2>/dev/null || true
    python3 - "$OUT/receipt.json" "$TIP" "${ARKS_JOURNEY_IMAGE_TAG:-arks-verify-m1:0.1.0}" <<'PY'
import json, sys, time
out, tip, image = sys.argv[1:]
receipt = {
    "schema": "arks-hub-mutate-subset/v1",
    "gate": "IW-HUB-MUTATE",
    "disposition": "verified-product-path",
    "venue": "docker-dod-hub-matrix",
    "tip": tip,
    "image_tag": image,
    "claimed_mutate": [
        "issue close", "issue reopen", "issue edit", "issue comment",
        "pr close", "pr reopen", "pr edit", "pr comment", "pr merge",
        "release view", "label edit", "label delete",
    ],
    "checks": [
        "HF-MUT peer-faithful mutate/view against local forge",
        "HF-MUT STRICT greens (ARKS_HUB_IW_MUTATE=1)",
    ],
    "note": "Actions execute / assets / pr checkout stay Partial; stub exit-2 ≠ Claimed.",
    "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
}
with open(out, "w", encoding="utf-8") as f:
    json.dump(receipt, f, indent=2)
    f.write("\n")
PY
    echo "hub-gh-arks-matrix-e2e: receipt=$OUT/receipt.json"
  else
    echo "hub-gh-arks-matrix-e2e: host run only; no .spec/evidence receipt (Docker Claimed venue required)"
  fi
  exit 0
fi

# --- IW-HUB-FULL expand + Missing→0 (ARKS_HUB_IW_FULL=1) ---
# Prefer honest Partial for non-expand leaves; Claimed expand list only with receipt.
if [[ "${ARKS_HUB_IW_FULL:-0}" == "1" ]]; then
  echo "hub-gh-arks-matrix-e2e: IW-HUB-FULL path (sot:root/arks-remaining-incompletes-wave)"
  if [[ ! -f "$INCOMPLETES_SOT" ]]; then
    echo "FAIL: missing incompletes SoT $INCOMPLETES_SOT" >&2
    exit 1
  fi
  command -v python3 >/dev/null || {
    echo "FAIL: python3 missing for IW-HUB-FULL" >&2
    exit 1
  }
  if [[ "$missing" -gt 0 ]]; then
    echo "FAIL IW-HUB-FULL: v1 Missing=$missing (inventory bar requires Missing=0)" >&2
    if [[ "${ARKS_DOD_STRICT:-0}" == "1" ]]; then
      exit 1
    fi
    echo "hub-gh-arks-matrix-e2e: IW-HUB-FULL Partial — Missing>0"
    exit 0
  fi
  echo "hub-gh-arks-matrix-e2e: HF-FULL inventory Missing=0 ok"

  WORK="$(mktemp -d /tmp/arks-hub-full.XXXXXX)"
  SERVER_PID=""
  cleanup_full() {
    if [[ -n "${SERVER_PID:-}" ]] && kill -0 "$SERVER_PID" 2>/dev/null; then
      kill "$SERVER_PID" 2>/dev/null || true
      wait "$SERVER_PID" 2>/dev/null || true
    fi
    rm -rf "$WORK"
  }
  trap cleanup_full EXIT

  FORGE_PORT="$(python3 - <<'PY'
import socket
s = socket.socket()
s.bind(("127.0.0.1", 0))
print(s.getsockname()[1])
s.close()
PY
)"
  FORGE_LISTEN="127.0.0.1:${FORGE_PORT}"
  mkdir -p "$WORK/hub-data" "$WORK/home/.config/arks"
  export HOME="$WORK/home"
  export ARKS_HUB_TOKEN_FILE="$WORK/home/.config/arks/hub-token"
  export ARKS_HUB_TOKEN="local-dev-token"
  export ARKS_HUB_API="http://${FORGE_LISTEN}"
  export ARKS_HUB_REPO="local/hubfull"

  "$ARKS_BIN" serve --no-ssh --no-http \
    --forge-listen "$FORGE_LISTEN" \
    --forge-data "$WORK/hub-data" \
    . >"$WORK/server.out" 2>"$WORK/server.err" &
  SERVER_PID=$!

  python3 - "$FORGE_PORT" <<'PY'
import socket, sys, time
port = int(sys.argv[1])
for _ in range(200):
    try:
        with socket.create_connection(("127.0.0.1", port), timeout=.2):
            raise SystemExit(0)
    except OSError:
        time.sleep(.05)
raise SystemExit("hub-full forge listener not ready")
PY

  FULL_OK=1
  run_full() {
    local name="$1"
    shift
    local out ec
    set +e
    out="$("$@" 2>&1)"
    ec=$?
    set -e
    printf '%s\n' "$out" >"$WORK/${name}.out"
    if [[ "$ec" -ne 0 ]]; then
      echo "FAIL HF-FULL $name exit=$ec" >&2
      echo "$out" >&2
      FULL_OK=0
      return 0
    fi
    echo "hub-gh-arks-matrix-e2e: HF-FULL $name ok"
  }

  # Flat-10 + expand Claimed leaves (sot:root/arks-remaining-incompletes-wave).
  run_full auth-login "$ARKS_BIN" hub auth login
  run_full auth-status "$ARKS_BIN" hub auth status
  run_full auth-token "$ARKS_BIN" hub auth token
  run_full repo-create "$ARKS_BIN" hub repo create hubfull --description "IW-HUB-FULL"
  run_full repo-list "$ARKS_BIN" hub repo list
  run_full repo-view "$ARKS_BIN" hub repo view local/hubfull
  run_full issue-create "$ARKS_BIN" hub issue create --repo local/hubfull --title "full-issue" --body "HF"
  run_full issue-list "$ARKS_BIN" hub issue list --repo local/hubfull
  run_full issue-view "$ARKS_BIN" hub issue view 1 --repo local/hubfull
  run_full pr-create "$ARKS_BIN" hub pr create --repo local/hubfull --title "full-pr" --body "HF" --head feature --base main
  run_full pr-list "$ARKS_BIN" hub pr list --repo local/hubfull
  run_full pr-view "$ARKS_BIN" hub pr view 1 --repo local/hubfull
  run_full label-create "$ARKS_BIN" hub label create --repo local/hubfull --name bug --color ff0000 --description "bug"
  run_full label-list "$ARKS_BIN" hub label list --repo local/hubfull
  run_full release-create "$ARKS_BIN" hub release create --repo local/hubfull --tag v0.0.1 --title "r1" --notes "n"
  run_full release-list "$ARKS_BIN" hub release list --repo local/hubfull
  run_full org-list "$ARKS_BIN" hub org list
  run_full search-repos "$ARKS_BIN" hub search repos hubfull
  run_full search-issues "$ARKS_BIN" hub search issues full-issue
  run_full search-prs "$ARKS_BIN" hub search prs full-p
  run_full auth-logout "$ARKS_BIN" hub auth logout

  if [[ "$FULL_OK" -eq 1 ]]; then
    if ! grep -Fq 'bug' "$WORK/label-list.out" \
      || ! grep -Fq 'v0.0.1' "$WORK/release-list.out" \
      || ! grep -Fq 'local' "$WORK/org-list.out" \
      || ! grep -Fq 'hubfull' "$WORK/search-repos.out" \
      || ! grep -Fq 'full-issue' "$WORK/search-issues.out" \
      || ! grep -Fq 'full-pr' "$WORK/search-prs.out"; then
      echo "FAIL HF-FULL peer-faithful expand shapes" >&2
      FULL_OK=0
    else
      echo "hub-gh-arks-matrix-e2e: HF-FULL peer-faithful expand shapes ok"
    fi
  fi

  if [[ "$FULL_OK" -ne 1 ]]; then
    cat "$WORK/server.err" >&2 || true
    if [[ "${ARKS_DOD_STRICT:-0}" == "1" ]]; then
      echo "hub-gh-arks-matrix-e2e: FAIL STRICT IW-HUB-FULL expand (no invent PASS)" >&2
      exit 1
    fi
    echo "hub-gh-arks-matrix-e2e: IW-HUB-FULL expand Partial/failed"
    exit 0
  fi

  echo "hub-gh-arks-matrix-e2e: HF-FULL expand greens (v1 Missing=0)"

  if [[ -f /.dockerenv ]]; then
    TS="$(date -u +%Y%m%d%H%M%S)"
    OUT=".spec/evidence/${TS}-hub-full-expand"
    mkdir -p "$OUT"
    cp "$WORK/server.err" "$OUT/server.err" 2>/dev/null || true
    for f in auth-login auth-status auth-token auth-logout repo-create repo-list repo-view \
      issue-create issue-list issue-view pr-create pr-list pr-view \
      label-create label-list release-create release-list org-list \
      search-repos search-issues search-prs; do
      cp "$WORK/${f}.out" "$OUT/${f}.out" 2>/dev/null || true
    done
    python3 - "$OUT/receipt.json" "$TIP" "${ARKS_JOURNEY_IMAGE_TAG:-arks-verify-m1:0.1.0}" <<'PY'
import json, sys, time
out, tip, image = sys.argv[1:]
receipt = {
    "schema": "arks-hub-full-expand/v1",
    "gate": "IW-HUB-FULL",
    "disposition": "verified-product-path",
    "venue": "docker-dod-hub-matrix",
    "tip": tip,
    "image_tag": image,
    "v1_missing": 0,
    "claimed_expand": [
        "auth logout", "auth token",
        "repo view",
        "label list", "label create",
        "release list", "release create",
        "org list",
        "search issues", "search prs", "search repos",
    ],
    "claimed_flat_subset_retained": [
        "auth login", "auth status",
        "repo create", "repo list",
        "issue create", "issue list", "issue view",
        "pr create", "pr list", "pr view",
    ],
    "checks": [
        "HF-FULL inventory v1 Missing=0",
        "HF-FULL expand leaves peer-faithful against local forge",
        "HF-FULL STRICT greens (ARKS_HUB_IW_FULL=1)",
    ],
    "note": "Non-expand v1 leaves may remain Partial; inventing Claimed for stub exit-2 is forbidden.",
    "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
}
with open(out, "w", encoding="utf-8") as f:
    json.dump(receipt, f, indent=2)
    f.write("\n")
PY
    echo "hub-gh-arks-matrix-e2e: receipt=$OUT/receipt.json"
  else
    echo "hub-gh-arks-matrix-e2e: host run only; no .spec/evidence receipt (Docker Claimed venue required)"
  fi
  exit 0
fi

# --- IW-HUB-FLAT Claimed subset (ARKS_HUB_IW_FLAT=1) ---
# Does not invent full-CSV Missing=0 Claimed. B3 ≠ this path.
if [[ "${ARKS_HUB_IW_FLAT:-0}" == "1" ]]; then
  echo "hub-gh-arks-matrix-e2e: IW-HUB-FLAT subset path (sot:root/arks-verify-docker-e2e-claim-wave)"
  if [[ ! -f "$DOCKER_SOT" ]]; then
    echo "FAIL: missing docker claim-wave SoT $DOCKER_SOT" >&2
    exit 1
  fi
  command -v python3 >/dev/null || {
    echo "FAIL: python3 missing for IW-HUB-FLAT" >&2
    exit 1
  }

  # HF-DISC: flat forge leaves discoverable (not only nested hub).
  flat_pr="$("$ARKS_BIN" pr --help 2>&1 || true)"
  flat_issue="$("$ARKS_BIN" issue --help 2>&1 || true)"
  if ! echo "$flat_pr" | grep -Eqi 'pr|pull'; then
    echo "FAIL HF-DISC: arks pr --help not discoverable" >&2
    exit 1
  fi
  if ! echo "$flat_issue" | grep -Eqi 'issue'; then
    echo "FAIL HF-DISC: arks issue --help not discoverable" >&2
    exit 1
  fi
  echo "hub-gh-arks-matrix-e2e: HF-DISC ok — flat pr/issue discoverable"

  WORK="$(mktemp -d /tmp/arks-hub-flat.XXXXXX)"
  SERVER_PID=""
  cleanup_flat() {
    if [[ -n "${SERVER_PID:-}" ]] && kill -0 "$SERVER_PID" 2>/dev/null; then
      kill "$SERVER_PID" 2>/dev/null || true
      wait "$SERVER_PID" 2>/dev/null || true
    fi
    rm -rf "$WORK"
  }
  trap cleanup_flat EXIT

  FORGE_PORT="$(python3 - <<'PY'
import socket
s = socket.socket()
s.bind(("127.0.0.1", 0))
print(s.getsockname()[1])
s.close()
PY
)"
  FORGE_LISTEN="127.0.0.1:${FORGE_PORT}"
  mkdir -p "$WORK/hub-data" "$WORK/home/.config/arks"
  export HOME="$WORK/home"
  export ARKS_HUB_TOKEN_FILE="$WORK/home/.config/arks/hub-token"
  export ARKS_HUB_TOKEN="local-dev-token"
  export ARKS_HUB_API="http://${FORGE_LISTEN}"
  export ARKS_HUB_REPO="local/hubflat"

  "$ARKS_BIN" serve --no-ssh --no-http \
    --forge-listen "$FORGE_LISTEN" \
    --forge-data "$WORK/hub-data" \
    . >"$WORK/server.out" 2>"$WORK/server.err" &
  SERVER_PID=$!

  python3 - "$FORGE_PORT" <<'PY'
import socket, sys, time
port = int(sys.argv[1])
for _ in range(200):
    try:
        with socket.create_connection(("127.0.0.1", port), timeout=.2):
            raise SystemExit(0)
    except OSError:
        time.sleep(.05)
raise SystemExit("hub-flat forge listener not ready")
PY

  SUBSET_OK=1
  run_flat() {
    local name="$1"
    shift
    local out ec
    set +e
    out="$("$@" 2>&1)"
    ec=$?
    set -e
    printf '%s\n' "$out" >"$WORK/${name}.out"
    if [[ "$ec" -ne 0 ]]; then
      echo "FAIL HF-SUBSET $name exit=$ec" >&2
      echo "$out" >&2
      SUBSET_OK=0
      return 0
    fi
    echo "hub-gh-arks-matrix-e2e: HF-SUBSET $name ok"
  }

  # Claimed subset (normative): auth/repo/issue/pr create|list|view via nested + flat.
  run_flat auth-login "$ARKS_BIN" hub auth login
  run_flat auth-status "$ARKS_BIN" hub auth status
  run_flat repo-create "$ARKS_BIN" hub repo create hubflat --description "IW-HUB-FLAT"
  run_flat repo-list "$ARKS_BIN" hub repo list
  run_flat issue-create-hub "$ARKS_BIN" hub issue create --repo local/hubflat --title "flat-issue" --body "HF"
  run_flat issue-list-flat "$ARKS_BIN" issue list --repo local/hubflat
  run_flat issue-view-flat "$ARKS_BIN" issue view 1 --repo local/hubflat
  run_flat pr-create-flat "$ARKS_BIN" pr create --repo local/hubflat --title "flat-pr" --body "HF" --head feature --base main
  run_flat pr-list-hub "$ARKS_BIN" hub pr list --repo local/hubflat
  run_flat pr-view-flat "$ARKS_BIN" pr view 1 --repo local/hubflat

  if [[ "$SUBSET_OK" -eq 1 ]]; then
    if ! grep -Fq 'flat-issue' "$WORK/issue-list-flat.out" \
      || ! grep -Eq '"number"[[:space:]]*:[[:space:]]*1' "$WORK/issue-view-flat.out" \
      || ! grep -Fq 'flat-pr' "$WORK/pr-list-hub.out" \
      || ! grep -Eq '"number"[[:space:]]*:[[:space:]]*1' "$WORK/pr-view-flat.out"; then
      echo "FAIL HF-SUBSET peer-faithful JSON/title shape" >&2
      SUBSET_OK=0
    else
      echo "hub-gh-arks-matrix-e2e: HF-SUBSET peer-faithful shapes ok"
    fi
  fi

  if [[ "$SUBSET_OK" -ne 1 ]]; then
    cat "$WORK/server.err" >&2 || true
    if [[ "${ARKS_DOD_STRICT:-0}" == "1" ]]; then
      echo "hub-gh-arks-matrix-e2e: FAIL STRICT IW-HUB-FLAT subset (no invent PASS)" >&2
      exit 1
    fi
    echo "hub-gh-arks-matrix-e2e: IW-HUB-FLAT subset Partial/failed; full-matrix Missing=$missing unchanged"
    exit 0
  fi

  echo "hub-gh-arks-matrix-e2e: HF-STRICT subset greens (full CSV Missing=$missing may remain — not invent PASS)"

  if [[ -f /.dockerenv ]]; then
    TS="$(date -u +%Y%m%d%H%M%S)"
    OUT=".spec/evidence/${TS}-hub-flat-subset"
    mkdir -p "$OUT"
    cp "$WORK/server.err" "$OUT/server.err" 2>/dev/null || true
    for f in auth-login auth-status repo-create repo-list issue-create-hub issue-list-flat issue-view-flat pr-create-flat pr-list-hub pr-view-flat; do
      cp "$WORK/${f}.out" "$OUT/${f}.out" 2>/dev/null || true
    done
    python3 - "$OUT/receipt.json" "$TIP" "${ARKS_JOURNEY_IMAGE_TAG:-arks-verify-m1:0.1.0}" "$missing" <<'PY'
import json, sys, time
out, tip, image, missing = sys.argv[1:]
receipt = {
    "schema": "arks-hub-flat-subset/v1",
    "gate": "IW-HUB-FLAT",
    "disposition": "verified-product-path",
    "venue": "docker-dod-hub-matrix",
    "tip": tip,
    "image_tag": image,
    "claimed_subset": [
        "auth login", "auth status",
        "repo create", "repo list",
        "issue create", "issue list", "issue view",
        "pr create", "pr list", "pr view",
    ],
    "checks": [
        "HF-DISC flat pr/issue discoverable",
        "HF-SUBSET nested+flat create/list/view against local forge",
        "HF-STRICT subset greens (ARKS_HUB_IW_FLAT=1)",
    ],
    "full_csv_v1_missing": int(missing),
    "note": "B3 hub HTTPS remote Claimed alone does not satisfy IW-HUB-FLAT; full-matrix Missing!=0 is not invent PASS.",
    "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
}
with open(out, "w", encoding="utf-8") as f:
    json.dump(receipt, f, indent=2)
    f.write("\n")
PY
    echo "hub-gh-arks-matrix-e2e: receipt=$OUT/receipt.json"
  else
    echo "hub-gh-arks-matrix-e2e: host run only; no .spec/evidence receipt (Docker Claimed venue required)"
  fi
  exit 0
fi

if [[ "${ARKS_DOD_STRICT:-0}" == "1" ]]; then
  if [[ "$missing" -gt 0 ]]; then
    echo "hub-gh-arks-matrix-e2e: FAIL STRICT — v1 Missing=$missing (no invent PASS)"
    exit 1
  fi
  fail=0
  while IFS=, read -r col1 col2 col3 col4 _rest; do
    [[ "$col1" == "gh_command" || "$col1" == $'\ufeffgh_pin' || "$col1" == "gh_pin" ]] && continue
    if echo "$header" | grep -q 'status_now'; then
      cmd="$col1"; target="$col2"; phase="$col4"
    else
      cmd="$col2"; target="$col3"; phase="$col4"
    fi
    [[ "$phase" == "v1" ]] || continue
    case "$cmd" in
      *' '*|\"*\") continue ;;
    esac
    cmd="${cmd//\"/}"
    [[ -z "$cmd" || "$target" == "OUT" ]] && continue
    out="$("$ARKS_BIN" hub "$cmd" 2>&1 || true)"
    if echo "$out" | grep -qi 'unknown command'; then
      echo "FAIL: arks hub $cmd unknown under STRICT"
      fail=1
    fi
  done < "$CSV"
  if [[ "$fail" -ne 0 ]]; then
    exit 1
  fi
  if [[ ! -f "$SCENARIO" ]]; then
    echo "hub-gh-arks-matrix-e2e: FAIL STRICT — missing local multi-agent scenario SoT $SCENARIO"
    exit 1
  fi
  if [[ ! -f "$UMBRELLA" ]]; then
    echo "hub-gh-arks-matrix-e2e: FAIL STRICT — missing umbrella SoT $UMBRELLA"
    exit 1
  fi
  echo "hub-gh-arks-matrix-e2e: PASS STRICT (v1 Missing=0; groups recognized; scenario+umbrella SoT present)"
  exit 0
fi

if [[ -f "$SCENARIO" ]]; then
  echo "hub-gh-arks-matrix-e2e: scenario SoT present ($SCENARIO) — framework scaffold; runner Claimed separately"
fi
if [[ -f "$UMBRELLA" ]]; then
  echo "hub-gh-arks-matrix-e2e: umbrella SoT ok ($UMBRELLA)"
fi
echo "hub-gh-arks-matrix-e2e: smoke-only PASS (SoT+CSV+help); Claimed hub e2e waits ARKS_DOD_STRICT=1 + evidence"
echo "hub-gh-arks-matrix-e2e: IW-HUB-FLAT subset Claimed path: ARKS_HUB_IW_FLAT=1 (+ STRICT)"
echo "hub-gh-arks-matrix-e2e: IW-HUB-FULL expand Claimed path: ARKS_HUB_IW_FULL=1 (+ STRICT); v1 Missing=$missing"
exit 0
