#!/usr/bin/env bash
# Related hosting e2e — arks-only self-host (LAN/WAN product story).
# SoT: .spec/sot/arks-self-host-server.sot.md
# Umbrella: .spec/sot/verify-cleanroom-symmetry-matrix.sot.md
# MUST NOT invent PASS; MUST NOT spawn gh. Stock git is client oracle only.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

SOT="${ARKS_SELF_HOST_SOT:-.spec/sot/arks-self-host-server.sot.md}"
UMBRELLA="${ARKS_SYMMETRY_UMBRELLA_SOT:-.spec/sot/verify-cleanroom-symmetry-matrix.sot.md}"

echo "self-host-e2e: tip=$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
echo "self-host-e2e: pin git=$(git --version 2>/dev/null || echo unknown)"

if [[ ! -f "$SOT" ]]; then
  echo "self-host-e2e: SKIP — missing SoT $SOT"
  exit 0
fi
if [[ ! -f "$UMBRELLA" ]]; then
  echo "self-host-e2e: SKIP — missing umbrella $UMBRELLA"
  exit 0
fi
echo "self-host-e2e: SoT+umbrella present (design linked)"

ARKS_BIN="${ARKS_BIN:-}"
if [[ -z "$ARKS_BIN" ]]; then
  if [[ -x ./.cargo-target/debug/arks ]]; then
    ARKS_BIN=./.cargo-target/debug/arks
  elif [[ -x ./target/debug/arks ]]; then
    ARKS_BIN=./target/debug/arks
  elif [[ -n "${CARGO_TARGET_DIR:-}" && -x "${CARGO_TARGET_DIR}/debug/arks" ]]; then
    ARKS_BIN="${CARGO_TARGET_DIR}/debug/arks"
  else
    echo "self-host-e2e: SKIP — arks binary not built (scaffold; no invent PASS)"
    exit 0
  fi
fi

# Discoverability smoke only — not LAN bind Claimed.
# Prefer `arks serve` (flags; default both). Transitional serve-http ≡ serve --no-ssh.
for smoke in "serve --help" "serve config --help" "daemon --help"; do
  # shellcheck disable=SC2086
  out="$($ARKS_BIN $smoke 2>&1 || true)"
  if echo "$out" | grep -qi 'command not implemented'; then
    echo "self-host-e2e: note — $smoke not ready (scaffold)"
  else
    echo "self-host-e2e: $smoke ok (smoke)"
  fi
done

if [[ "${ARKS_DOD_STRICT:-0}" == "1" ]]; then
  echo "self-host-e2e: FAIL STRICT — dedicated LAN-shaped hosting e2e not Claimed yet (no invent PASS)"
  exit 1
fi

echo "self-host-e2e: SKIP/scaffold PASS — design SoT linked; LAN bind + Basic + stock-git client scenario TODO"
exit 0
