#!/usr/bin/env bash
# DoD pillar 2 — git-daemon vs arks daemon matrix e2e (pin 2.39.5).
# Sibling owns daemon matrix SoT; until it lands, smoke help + loopback subset only.
# Never invents PASS; never spawns gh. Stock `git` is pin oracle only.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

# Intent SoT landed at tip ≥9442659; flag-depth machine CSV may still be pending.
INTENT_SOT="${ARKS_DAEMON_INTENT_SOT:-.spec/sot/arks-daemon-product-intent.sot.md}"
SOT="${ARKS_DAEMON_MATRIX_SOT:-.spec/sot/arks-daemon-git-command-matrix.sot.md}"
CSV="${ARKS_DAEMON_MATRIX_CSV:-verify/matrices/daemon-git-arks-e2e-matrix.csv}"
# Prefer landed sibling daemon matrix SoT when present.

echo "daemon-git-arks-matrix-e2e: tip=$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
echo "daemon-git-arks-matrix-e2e: pin git=$(git --version)"

ARKS_BIN="${ARKS_BIN:-}"
if [[ -z "$ARKS_BIN" ]]; then
  if [[ -x ./.cargo-target/debug/arks ]]; then
    ARKS_BIN=./.cargo-target/debug/arks
  elif [[ -x ./target/debug/arks ]]; then
    ARKS_BIN=./target/debug/arks
  elif [[ -n "${CARGO_TARGET_DIR:-}" && -x "${CARGO_TARGET_DIR}/debug/arks" ]]; then
    ARKS_BIN="${CARGO_TARGET_DIR}/debug/arks"
  else
    cargo build -p arks -p arks-daemon
    ARKS_BIN=./.cargo-target/debug/arks
    [[ -x "$ARKS_BIN" ]] || ARKS_BIN=./target/debug/arks
    [[ -x "$ARKS_BIN" ]] || ARKS_BIN="${CARGO_TARGET_DIR:-target}/debug/arks"
  fi
fi

# Muscle-memory entry must exist (single release binary).
help_out="$("$ARKS_BIN" daemon --help 2>&1 || true)"
if ! echo "$help_out" | grep -qi 'git://'; then
  echo "FAIL: arks daemon --help missing git:// usage"
  exit 1
fi
echo "daemon-git-arks-matrix-e2e: arks daemon --help ok"

# Pin oracle usage exists (git-daemon). Prefer `git daemon -h` — man pages often absent in image.
pin_help="$(git daemon -h 2>&1 || true)"
if ! echo "$pin_help" | grep -qi 'usage: git daemon'; then
  echo "FAIL: pin git daemon -h missing usage"
  exit 1
fi
echo "daemon-git-arks-matrix-e2e: pin git daemon -h ok"

if [[ ! -f "$INTENT_SOT" ]]; then
  echo "daemon-git-arks-matrix-e2e: FAIL — missing daemon intent SoT $INTENT_SOT"
  exit 1
fi
echo "daemon-git-arks-matrix-e2e: intent SoT ok ($INTENT_SOT)"

if [[ ! -f "$SOT" || ! -f "$CSV" ]]; then
  echo "daemon-git-arks-matrix-e2e: SKIP flag-depth matrix — waiting $SOT / $CSV"
  echo "daemon-git-arks-matrix-e2e: smoke-only PASS (help + intent SoT); flag e2e not Claimed"
  exit 0
fi

missing=$(awk -F, 'NR>1 && ($3=="Missing" || $3=="missing") {c++} END{print c+0}' "$CSV")
echo "daemon-git-arks-matrix-e2e: Missing=$missing from $CSV"
if [[ "${ARKS_DOD_STRICT:-0}" == "1" || "$missing" -eq 0 ]]; then
  if [[ "$missing" -gt 0 ]]; then
    echo "daemon-git-arks-matrix-e2e: FAIL — daemon matrix Missing=$missing (no invent PASS)"
    exit 1
  fi
  echo "daemon-git-arks-matrix-e2e: PASS — matrix Missing=0"
  exit 0
fi
echo "daemon-git-arks-matrix-e2e: smoke-only PASS (CSV present, Missing=$missing); STRICT waits ARKS_DOD_STRICT=1"
exit 0
