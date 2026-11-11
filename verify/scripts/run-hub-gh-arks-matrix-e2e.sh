#!/usr/bin/env bash
# DoD pillar 3 — gh vs arks hub matrix e2e + local multi-agent PR scenarios.
# Entry is `arks hub` (single release binary). MUST NOT spawn `gh` or `git`.
# Sibling matrix SoT: .spec/sot/arks-hub-gh-command-matrix.sot.md (+ CSV beside it).
# IW-HUB-FLAT Claimed subset: sot:root/arks-verify-docker-e2e-claim-wave (ARKS_HUB_IW_FLAT=1).
# B3 client remote ≠ flat-leaf Claimed.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

SOT="${ARKS_HUB_MATRIX_SOT:-.spec/sot/arks-hub-gh-command-matrix.sot.md}"
DOCKER_SOT="${ARKS_DOCKER_E2E_CLAIM_SOT:-.spec/sot/arks-verify-docker-e2e-claim-wave.sot.md}"
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

header="$(head -n1 "$CSV" || true)"
if echo "$header" | grep -q 'status_now'; then
  # Living e2e: gh_command,arks_hub_target,status_now,phase,notes
  v1=$(awk -F, 'NR>1 && $4=="v1" {c++} END{print c+0}' "$CSV")
  missing=$(awk -F, 'NR>1 && $4=="v1" && $3=="Missing" {c++} END{print c+0}' "$CSV")
  refuse=$(awk -F, 'NR>1 && $3=="Refuse/N/A" {c++} END{print c+0}' "$CSV")
  echo "hub-gh-arks-matrix-e2e: csv=$CSV v1=$v1 Missing(v1)=$missing Refuse=$refuse"
else
  # Design CSV: gh_pin,gh_command,arks_hub_target,phase,notes
  v1=$(awk -F, 'NR>1 && $4=="v1" {c++} END{print c+0}' "$CSV")
  missing=0
  refuse=$(awk -F, 'NR>1 && ($3=="OUT" || $4=="out-of-scope") {c++} END{print c+0}' "$CSV")
  echo "hub-gh-arks-matrix-e2e: csv=$CSV v1=$v1 OUT=$refuse (design table)"
fi

if [[ "$v1" -eq 0 ]]; then
  echo "FAIL: hub matrix CSV has no v1 rows"
  exit 1
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
exit 0
