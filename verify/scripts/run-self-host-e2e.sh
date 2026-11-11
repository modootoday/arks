#!/usr/bin/env bash
# Related hosting e2e — arks-only self-host (LAN/WAN product story).
# SoT: .spec/sot/arks-self-host-server.sot.md
# Partial residual: .spec/sot/arks-partial-residual-wave.sot.md (IW-FORGE-SERVE)
# Research: .spec/research/20261111093543-partial-residual-wave-web-research.research.md
# Umbrella: .spec/sot/verify-cleanroom-symmetry-matrix.sot.md
# MUST NOT invent PASS; MUST NOT spawn gh. Stock git is client oracle only.
# Claimed bar (future): PR create/list + issue create/list against in-process forge
# under `arks serve` + scrubbed receipt — B3 client remote ≠ this slot.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

SOT="${ARKS_SELF_HOST_SOT:-.spec/sot/arks-self-host-server.sot.md}"
PARTIAL_SOT="${ARKS_PARTIAL_RESIDUAL_SOT:-.spec/sot/arks-partial-residual-wave.sot.md}"
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
if [[ ! -f "$PARTIAL_SOT" ]]; then
  echo "self-host-e2e: SKIP — missing Partial residual SoT $PARTIAL_SOT"
  exit 0
fi
echo "self-host-e2e: SoT+umbrella+partial-residual present (IW-FORGE-SERVE Partial gate)"

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

# Discoverability smoke only — not LAN bind Claimed / not hosting e2e Claimed.
# Prefer `arks serve` (flags; default HTTP+SSH+forge). Transitional serve-http ≡ serve --no-ssh --no-forge.
COHOST_OK=0
for smoke in "serve --help" "serve config --help" "daemon --help"; do
  # shellcheck disable=SC2086
  out="$($ARKS_BIN $smoke 2>&1 || true)"
  if echo "$out" | grep -qi 'command not implemented'; then
    echo "self-host-e2e: note — $smoke not ready (scaffold)"
  else
    echo "self-host-e2e: $smoke ok (smoke)"
  fi
  if [[ "$smoke" == "serve --help" ]]; then
    # IW-FORGE-SERVE Partial: default cohost must advertise forge opt-out / cohost honesty.
    if echo "$out" | grep -Eqi -- '--no-forge' \
      && echo "$out" | grep -Eqi 'forge'; then
      COHOST_OK=1
      echo "self-host-e2e: IW-FORGE-SERVE Partial — serve --help advertises forge cohost / --no-forge"
    else
      echo "self-host-e2e: note — serve --help missing forge cohost / --no-forge markers (Partial deepen gap)"
    fi
  fi
done

if [[ "$COHOST_OK" -ne 1 ]]; then
  echo "self-host-e2e: note — forge cohost discoverability Partial gap (not Claimed invent)"
fi

# Hosting e2e Claimed journeys (PR + issue against local forge module) are still Missing.
echo "self-host-e2e: IW-FORGE-SERVE — hosting e2e PR/issue host journeys still Missing (B3 ≠ this row)"

if [[ "${ARKS_DOD_STRICT:-0}" == "1" ]]; then
  echo "self-host-e2e: FAIL STRICT — hosting e2e Claimed (PR+issue host journeys) not evidenced (no invent PASS)"
  exit 1
fi

echo "self-host-e2e: SKIP/scaffold PASS — design SoT linked; LAN bind + Basic + PR/issue host journeys TODO"
exit 0
