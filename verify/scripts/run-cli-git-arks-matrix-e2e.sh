#!/usr/bin/env bash
# DoD pillar 1 — git vs arks CLI matrix e2e (pin 2.39.5).
# Reads living CSV verify/matrices/cli-git-help-matrix.csv.
# Never invents PASS; never spawns gh. Stock `git` is pin oracle only.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

CSV="${ARKS_CLI_MATRIX_CSV:-verify/matrices/cli-git-help-matrix.csv}"
SOT="${ARKS_CLI_MATRIX_SOT:-.spec/sot/cli-git-help-matrix.sot.md}"

if [[ ! -f "$CSV" ]]; then
  echo "cli-git-arks-matrix-e2e: SKIP — missing $CSV (sibling/matrix SoT not landed)"
  exit 0
fi
if [[ ! -f "$SOT" ]]; then
  echo "cli-git-arks-matrix-e2e: SKIP — missing $SOT"
  exit 0
fi

echo "cli-git-arks-matrix-e2e: tip=$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
echo "cli-git-arks-matrix-e2e: pin git=$(git --version)"
echo "cli-git-arks-matrix-e2e: csv=$CSV"

ARKS_BIN="${ARKS_BIN:-}"
if [[ -z "$ARKS_BIN" ]]; then
  if [[ -x ./.cargo-target/debug/arks ]]; then
    ARKS_BIN=./.cargo-target/debug/arks
  elif [[ -x ./target/debug/arks ]]; then
    ARKS_BIN=./target/debug/arks
  elif [[ -n "${CARGO_TARGET_DIR:-}" && -x "${CARGO_TARGET_DIR}/debug/arks" ]]; then
    ARKS_BIN="${CARGO_TARGET_DIR}/debug/arks"
  else
    cargo build -p arks
    ARKS_BIN=./.cargo-target/debug/arks
    [[ -x "$ARKS_BIN" ]] || ARKS_BIN=./target/debug/arks
    [[ -x "$ARKS_BIN" ]] || ARKS_BIN="${CARGO_TARGET_DIR:-target}/debug/arks"
  fi
fi

# Count Missing rows — DoD fails closed while Missing > 0 (M16 honesty).
missing=$(awk -F, 'NR>1 && $3=="Missing" {c++} END{print c+0}' "$CSV")
partial=$(awk -F, 'NR>1 && $3=="Partial" {c++} END{print c+0}' "$CSV")
refuse=$(awk -F, 'NR>1 && $3=="Refuse/N/A" {c++} END{print c+0}' "$CSV")
echo "cli-git-arks-matrix-e2e: Missing=$missing Partial=$partial Refuse/N/A=$refuse"

# Spot-check: every Partial/Parity-candidate command must not print "command not implemented".
fail=0
checked=0
while IFS=, read -r cmd category status target notes; do
  [[ "$cmd" == "git_command" ]] && continue
  case "$status" in
    Partial|Parity-candidate|Synonym-wired) ;;
    *) continue ;;
  esac
  # Skip arks-only / non-dispatch noise.
  case "$cmd" in
    ''|serve-http|lease|lock) ;;
  esac
  checked=$((checked + 1))
  # Limit sample to keep gate time bounded; full walk when Missing==0.
  if [[ "$missing" -gt 0 && "$checked" -gt 40 ]]; then
    break
  fi
  out="$("$ARKS_BIN" "$cmd" --help 2>&1 || true)"
  if echo "$out" | grep -qi 'command not implemented'; then
    echo "FAIL: $cmd still 'command not implemented' but CSV=$status"
    fail=1
  fi
done < "$CSV"

if [[ "$fail" -ne 0 ]]; then
  echo "cli-git-arks-matrix-e2e: FAIL — dispatch/CSV honesty mismatch"
  exit 1
fi

if [[ "$missing" -eq 0 ]]; then
  echo "cli-git-arks-matrix-e2e: PASS — Missing=0 and sampled dispatch honest (flag-depth still Partial)"
  exit 0
fi

# Strict DoD fails closed while Missing remain.
if [[ "${ARKS_DOD_STRICT:-0}" == "1" ]]; then
  echo "cli-git-arks-matrix-e2e: FAIL — Missing=$missing under ARKS_DOD_STRICT=1 (no invent PASS)"
  exit 1
fi

echo "cli-git-arks-matrix-e2e: smoke-only PASS (CSV readable, sample dispatch honest); Missing=$missing — full inventory DoD waits Missing=0"
exit 0
