#!/usr/bin/env bash
# Pre-release symmetry gate (pin DoD + pin inventory continuity).
# Cite: sot:root/release-symmetry-precheck + sot:root/verify-cleanroom-symmetry-matrix
#
# Release-blocking: pillar hard FAIL; CLI/daemon Missing>0; untracked pin leaves.
# Advisory: Partial rows; hub v1 Missing unless ARKS_RELEASE_BLOCK_HUB_MISSING=1.
# Does not invent Claimed/PASS. Uses existing run-symmetry-*.sh entrypoints.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

OUT_DIR="${ARKS_INVENTORY_OUT:-verify/out/symmetry-prerelease}"
mkdir -p "$OUT_DIR"
: >"$OUT_DIR/prerelease-summary.txt"
log() { echo "$*" | tee -a "$OUT_DIR/prerelease-summary.txt"; }

export ARKS_RELEASE_GATE=1
# Do not set ARKS_DOD_STRICT=1 globally — hub v1 Missing would invent a forever-red
# release while rows remain Partial stubs. Pin Missing is enforced below.
export ARKS_DOD_STRICT="${ARKS_DOD_STRICT:-0}"

CLI_CSV="${ARKS_CLI_MATRIX_CSV:-verify/matrices/cli-git-help-matrix.csv}"
DAEMON_CSV="${ARKS_DAEMON_MATRIX_CSV:-verify/matrices/daemon-git-arks-e2e-matrix.csv}"
HUB_CSV_DEFAULT=verify/matrices/hub-gh-arks-e2e-matrix.csv
[[ -f "$HUB_CSV_DEFAULT" ]] || HUB_CSV_DEFAULT=.spec/sot/arks-hub-gh-command-matrix.csv
HUB_CSV="${ARKS_HUB_MATRIX_CSV:-$HUB_CSV_DEFAULT}"

log "=== release symmetry precheck (pin) ==="
log "tip=$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
log "git_oracle=$(git --version 2>/dev/null || echo missing)"
log "ARKS_DOD_STRICT=$ARKS_DOD_STRICT ARKS_RELEASE_BLOCK_HUB_MISSING=${ARKS_RELEASE_BLOCK_HUB_MISSING:-0}"

log "--- pillar 1: git ↔ arks ---"
bash "$ROOT/verify/scripts/run-symmetry-git.sh"

log "--- pillar 2: git-daemon ↔ arks daemon ---"
bash "$ROOT/verify/scripts/run-symmetry-daemon.sh"

log "--- pillar 3: gh ↔ arks hub (smoke; Missing policy separate) ---"
bash "$ROOT/verify/scripts/run-symmetry-hub.sh"

# Enforce pin Missing bars (even when pillar smoke mode would allow Missing>0).
cli_missing="$(awk -F, 'NR>1 && $3=="Missing" {c++} END{print c+0}' "$CLI_CSV")"
daemon_missing="$(awk -F, 'NR>1 && ($3=="Missing" || $3=="missing") {c++} END{print c+0}' "$DAEMON_CSV")"
hub_v1_missing=0
if [[ -f "$HUB_CSV" ]]; then
  header="$(head -n1 "$HUB_CSV" || true)"
  if echo "$header" | grep -q 'status_now'; then
    hub_v1_missing="$(awk -F, 'NR>1 && $4=="v1" && $3=="Missing" {c++} END{print c+0}' "$HUB_CSV")"
  fi
fi

log "counts: cli_Missing=$cli_missing daemon_Missing=$daemon_missing hub_v1_Missing=$hub_v1_missing"

fail=0
if [[ "$cli_missing" -gt 0 ]]; then
  log "FAIL: pin CLI Missing=$cli_missing (release-blocking)"
  fail=1
fi
if [[ "$daemon_missing" -gt 0 ]]; then
  log "FAIL: pin daemon Missing=$daemon_missing (release-blocking)"
  fail=1
fi
if [[ "$hub_v1_missing" -gt 0 ]]; then
  log "advisory: hub v1 Missing=$hub_v1_missing (inventoried; not Claimed)"
  if [[ "${ARKS_RELEASE_BLOCK_HUB_MISSING:-0}" == "1" ]]; then
    log "FAIL: hub v1 Missing=$hub_v1_missing under ARKS_RELEASE_BLOCK_HUB_MISSING=1"
    fail=1
  fi
fi

log "--- pin inventory continuity (git / git-daemon; gh if present) ---"
export ARKS_INVENTORY_MODE=pin
export ARKS_INVENTORY_OUT="$OUT_DIR/pin-inventory"
if ! bash "$ROOT/verify/scripts/run-symmetry-latest-inventory.sh"; then
  log "FAIL: pin inventory continuity"
  fail=1
fi

if [[ "$fail" -ne 0 ]]; then
  log "=== prerelease RESULT: FAIL ==="
  exit 1
fi
log "=== prerelease RESULT: OK (pin Missing=0; untracked=0; Partial/hub Missing may remain advisory) ==="
exit 0
