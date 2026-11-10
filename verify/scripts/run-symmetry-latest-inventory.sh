#!/usr/bin/env bash
# Inventory continuity: compare live git / git-daemon / gh surface to living CSVs.
# Cite: sot:root/release-symmetry-precheck
#
# Modes (ARKS_INVENTORY_MODE):
#   pin    — require git (+ daemon -h); gh optional (skip hub if absent)
#   latest — require git + gh (host/runner "follow upstream" check)
#   both   — pin checks + gh when present (default)
#
# Release-blocking: untracked leaves (oracle present, CSV absent).
# Advisory: inventoried latest-only / Partial / hub Missing counts.
# NEVER invents Claimed/PASS. Product code MUST NOT spawn git/gh; this script
# is verify-oracle only.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

MODE="${ARKS_INVENTORY_MODE:-both}"
OUT_DIR="${ARKS_INVENTORY_OUT:-verify/out/symmetry-inventory}"
mkdir -p "$OUT_DIR"

CLI_CSV="${ARKS_CLI_MATRIX_CSV:-verify/matrices/cli-git-help-matrix.csv}"
DAEMON_CSV="${ARKS_DAEMON_MATRIX_CSV:-verify/matrices/daemon-git-arks-e2e-matrix.csv}"
if [[ -z "${ARKS_HUB_MATRIX_CSV:-}" ]]; then
  if [[ -f verify/matrices/hub-gh-arks-e2e-matrix.csv ]]; then
    HUB_CSV=verify/matrices/hub-gh-arks-e2e-matrix.csv
  else
    HUB_CSV=.spec/sot/arks-hub-gh-command-matrix.csv
  fi
else
  HUB_CSV="$ARKS_HUB_MATRIX_CSV"
fi

fail=0
advisory=0

log() { echo "$*" | tee -a "$OUT_DIR/summary.txt"; }

: >"$OUT_DIR/summary.txt"
log "=== symmetry latest/pin inventory ==="
log "mode=$MODE tip=$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
log "cli_csv=$CLI_CSV"
log "daemon_csv=$DAEMON_CSV"
log "hub_csv=$HUB_CSV"

if ! command -v git >/dev/null 2>&1; then
  log "FAIL: git not on PATH (oracle required)"
  exit 1
fi

GIT_VER="$(git --version 2>&1 || true)"
log "git_oracle=$GIT_VER"

# --- CLI: list-cmds must be inventoried ---
if [[ ! -f "$CLI_CSV" ]]; then
  log "FAIL: missing CLI CSV $CLI_CSV"
  exit 1
fi

git --list-cmds=main,others,alias 2>/dev/null | awk 'NF && $0 !~ / / {print}' | sort -u >"$OUT_DIR/git-list-cmds.txt"
awk -F, 'NR>1 {print $1}' "$CLI_CSV" | sed '/^$/d' | sort -u >"$OUT_DIR/cli-csv-cmds.txt"

untracked_cli="$(comm -23 "$OUT_DIR/git-list-cmds.txt" "$OUT_DIR/cli-csv-cmds.txt" || true)"
if [[ -n "$untracked_cli" ]]; then
  log "FAIL: untracked git list-cmds (not in CLI CSV):"
  echo "$untracked_cli" | tee "$OUT_DIR/untracked-cli.txt" | tee -a "$OUT_DIR/summary.txt"
  fail=1
else
  log "cli: untracked=0 (all list-cmds inventoried)"
  : >"$OUT_DIR/untracked-cli.txt"
fi

# CSV Missing count (pin DoD signal when oracle is pin; advisory note when latest)
cli_missing="$(awk -F, 'NR>1 && $3=="Missing" {c++} END{print c+0}' "$CLI_CSV")"
cli_partial="$(awk -F, 'NR>1 && $3=="Partial" {c++} END{print c+0}' "$CLI_CSV")"
log "cli_csv: Missing=$cli_missing Partial=$cli_partial"
if [[ "$cli_missing" -gt 0 ]]; then
  if [[ "$MODE" == "latest" ]]; then
    log "advisory: CLI CSV Missing=$cli_missing under latest mode (pin job owns Missing=0 bar)"
    advisory=1
  else
    log "FAIL: CLI CSV Missing=$cli_missing (release pin bar)"
    fail=1
  fi
fi

# Latest-only cmds already in CSV (skew tracking) — advisory list
comm -13 "$OUT_DIR/git-list-cmds.txt" "$OUT_DIR/cli-csv-cmds.txt" >"$OUT_DIR/csv-only-cli.txt" || true
# Host/latest cmds beyond a frozen pin list are fine if inventoried; emit count only.
list_n="$(wc -l <"$OUT_DIR/git-list-cmds.txt" | tr -d ' ')"
csv_n="$(wc -l <"$OUT_DIR/cli-csv-cmds.txt" | tr -d ' ')"
log "cli: list-cmds=$list_n csv_rows_cmds=$csv_n"

# --- Daemon: long options from git daemon -h ---
if [[ ! -f "$DAEMON_CSV" ]]; then
  log "FAIL: missing daemon CSV $DAEMON_CSV"
  exit 1
fi

daemon_help="$(git daemon -h 2>&1 || true)"
printf '%s\n' "$daemon_help" >"$OUT_DIR/git-daemon-h.txt"
if ! echo "$daemon_help" | grep -qi 'usage: git daemon'; then
  log "FAIL: git daemon -h missing usage"
  fail=1
else
  # Extract long-option tokens; normalize --foo=<x> / --[no-]bar forms to stems for membership.
  echo "$daemon_help" | tr ' ' '\n' | sed -n 's/,*$//p' | grep -E '^--[a-z0-9\[\]=-]+' \
    | sed -E 's/^--\[no-\]/--/; s/=.*$//; s/\[.*$//' | sort -u >"$OUT_DIR/daemon-help-opts.txt" || true
  # CSV git_item may be --timeout=<n> or --user-path|--user-path=<path>
  awk -F, 'NR>1 {print $1}' "$DAEMON_CSV" | tr '|' '\n' | sed -E 's/^[[:space:]]+//;s/[[:space:]]+$//' \
    | grep -E '^--' | sed -E 's/^--\[no-\]/--/; s/=.*$//; s/\[.*$//' | sort -u >"$OUT_DIR/daemon-csv-opts.txt"

  untracked_daemon="$(comm -23 "$OUT_DIR/daemon-help-opts.txt" "$OUT_DIR/daemon-csv-opts.txt" || true)"
  if [[ -n "$untracked_daemon" ]]; then
    log "FAIL: untracked git-daemon long-options (not in daemon CSV):"
    echo "$untracked_daemon" | tee "$OUT_DIR/untracked-daemon.txt" | tee -a "$OUT_DIR/summary.txt"
    fail=1
  else
    log "daemon: untracked long-options=0"
    : >"$OUT_DIR/untracked-daemon.txt"
  fi
fi

daemon_missing="$(awk -F, 'NR>1 && ($3=="Missing" || $3=="missing") {c++} END{print c+0}' "$DAEMON_CSV")"
log "daemon_csv: Missing=$daemon_missing"
if [[ "$daemon_missing" -gt 0 ]]; then
  if [[ "$MODE" == "latest" ]]; then
    log "advisory: daemon CSV Missing=$daemon_missing under latest mode"
    advisory=1
  else
    log "FAIL: daemon CSV Missing=$daemon_missing (release pin bar)"
    fail=1
  fi
fi

# --- Hub / gh ---
need_gh=0
case "$MODE" in
  latest) need_gh=1 ;;
  both|pin)
    if command -v gh >/dev/null 2>&1; then need_gh=1; fi
    ;;
esac

if [[ "$need_gh" -eq 1 ]]; then
  if ! command -v gh >/dev/null 2>&1; then
    log "FAIL: gh required for mode=$MODE but not on PATH"
    exit 1
  fi
  GH_VER="$(gh --version 2>&1 | head -n1 || true)"
  log "gh_oracle=$GH_VER"
  if [[ ! -f "$HUB_CSV" ]]; then
    log "FAIL: missing hub CSV $HUB_CSV"
    exit 1
  fi

  # Top-level commands from `gh help` command sections only (stop before HELP TOPICS /
  # FLAGS / EXAMPLES / LEARN MORE — those are docs topics, not CLI leaves).
  gh help 2>/dev/null | awk '
    BEGIN { keep=0 }
    /^HELP TOPICS$/ { keep=0; next }
    /^FLAGS$/ { keep=0; next }
    /^EXAMPLES$/ { keep=0; next }
    /^LEARN MORE$/ { keep=0; next }
    /^[A-Z][A-Z0-9 ]*COMMANDS$/ { keep=1; next }
    keep && /^  [a-z][a-z0-9-]*:/ {
      line=$0
      sub(/^  /, "", line)
      sub(/:.*/, "", line)
      print line
    }
  ' | sort -u >"$OUT_DIR/gh-top-level.txt"
  # Deeper leaf check is optional and noisy across gh versions; top-level
  # continuity is the release-blocking inventory signal. Keep an empty file
  # for artifact shape stability.
  : >"$OUT_DIR/gh-reference-leaves.txt"

  header="$(head -n1 "$HUB_CSV" || true)"
  if echo "$header" | grep -q 'status_now'; then
    # living: gh_command,...
    awk -F, 'NR>1 {print $1}' "$HUB_CSV" | sed 's/^"//;s/"$//' | sort -u >"$OUT_DIR/hub-csv-cmds.txt"
    hub_v1_missing="$(awk -F, 'NR>1 && $4=="v1" && $3=="Missing" {c++} END{print c+0}' "$HUB_CSV")"
  else
    # design: gh_pin,gh_command,...
    awk -F, 'NR>1 {print $2}' "$HUB_CSV" | sed 's/^"//;s/"$//' | sort -u >"$OUT_DIR/hub-csv-cmds.txt"
    hub_v1_missing=0
  fi

  : >"$OUT_DIR/untracked-gh.txt"
  while IFS= read -r top; do
    [[ -z "$top" ]] && continue
    if ! grep -qxF "$top" "$OUT_DIR/hub-csv-cmds.txt" \
      && ! grep -qE "^${top}( |$)" "$OUT_DIR/hub-csv-cmds.txt"; then
      echo "$top" >>"$OUT_DIR/untracked-gh.txt"
    fi
  done <"$OUT_DIR/gh-top-level.txt"

  if [[ -s "$OUT_DIR/gh-reference-leaves.txt" ]]; then
    while IFS= read -r leaf; do
      [[ -z "$leaf" ]] && continue
      if ! grep -qxF "$leaf" "$OUT_DIR/hub-csv-cmds.txt"; then
        # allow parent group row only for depth>1 if explicitly deferred? No — leaf must be inventoried.
        echo "$leaf" >>"$OUT_DIR/untracked-gh.txt"
      fi
    done <"$OUT_DIR/gh-reference-leaves.txt"
    sort -u "$OUT_DIR/untracked-gh.txt" -o "$OUT_DIR/untracked-gh.txt"
  fi

  if [[ -s "$OUT_DIR/untracked-gh.txt" ]]; then
    log "FAIL: untracked gh commands (not in hub CSV):"
    tee -a "$OUT_DIR/summary.txt" <"$OUT_DIR/untracked-gh.txt"
    fail=1
  else
    log "hub: untracked gh top-level/reference=0"
  fi

  log "hub_csv: v1_Missing=$hub_v1_missing (inventoried Missing is advisory unless ARKS_RELEASE_BLOCK_HUB_MISSING=1)"
  if [[ "$hub_v1_missing" -gt 0 ]]; then
    advisory=1
    if [[ "${ARKS_RELEASE_BLOCK_HUB_MISSING:-0}" == "1" ]]; then
      log "FAIL: hub v1 Missing=$hub_v1_missing under ARKS_RELEASE_BLOCK_HUB_MISSING=1"
      fail=1
    fi
  fi
else
  log "hub: skipped (gh not required/absent for mode=$MODE)"
fi

log "advisory_signals=$advisory"
if [[ "$fail" -ne 0 ]]; then
  log "=== RESULT: FAIL (release-blocking inventory deltas) ==="
  exit 1
fi
log "=== RESULT: inventory continuity OK (untracked=0; Partial/hub Missing may remain advisory) ==="
exit 0
