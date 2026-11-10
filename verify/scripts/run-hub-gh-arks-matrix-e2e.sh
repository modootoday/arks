#!/usr/bin/env bash
# DoD pillar 3 — gh vs arks hub matrix e2e + local multi-agent PR scenarios.
# Entry is `arks hub` (single release binary). MUST NOT spawn `gh` or `git`.
# Sibling matrix SoT: .spec/sot/arks-hub-gh-command-matrix.sot.md (+ CSV beside it).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

SOT="${ARKS_HUB_MATRIX_SOT:-.spec/sot/arks-hub-gh-command-matrix.sot.md}"
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

echo "hub-gh-arks-matrix-e2e: tip=$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"

ARKS_BIN="${ARKS_BIN:-}"
if [[ -z "$ARKS_BIN" ]]; then
  if [[ -x ./.cargo-target/debug/arks ]]; then
    ARKS_BIN=./.cargo-target/debug/arks
  elif [[ -x ./target/debug/arks ]]; then
    ARKS_BIN=./target/debug/arks
  elif [[ -n "${CARGO_TARGET_DIR:-}" && -x "${CARGO_TARGET_DIR}/debug/arks" ]]; then
    ARKS_BIN="${CARGO_TARGET_DIR}/debug/arks"
  else
    cargo build -p arks -p arks-hub
    ARKS_BIN=./.cargo-target/debug/arks
    [[ -x "$ARKS_BIN" ]] || ARKS_BIN=./target/debug/arks
    [[ -x "$ARKS_BIN" ]] || ARKS_BIN="${CARGO_TARGET_DIR:-target}/debug/arks"
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
exit 0
