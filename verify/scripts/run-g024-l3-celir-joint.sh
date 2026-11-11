#!/usr/bin/env bash
# G24 — L3 HighConcurrency + CELIR index joint clean-room (M24 successor).
# SoT: l3-celir-index-joint-clean-room; matrix: verify/matrices/matrix-g024-l3-celir-joint.md
set -euo pipefail

ROOT="${ARKS_G24_ROOT:-/workspaces/gir}"
cd "$ROOT"

mkdir -p verify/gates/g024-l3-celir-joint
G24_LOCK="verify/gates/g024-l3-celir-joint/.run.lock.dir"
if ! mkdir "$G24_LOCK" 2>/dev/null; then
  if [[ -f "$G24_LOCK/owner" ]]; then
    age=$(( $(date +%s) - $(stat -c %Y "$G24_LOCK/owner" 2>/dev/null || echo 0) ))
    if [[ "$age" -gt 7200 ]]; then
      echo "G24: reclaiming stale lock (age=${age}s)"
      rm -rf "$G24_LOCK"
      mkdir "$G24_LOCK"
    else
      echo "G24: another run holds $G24_LOCK — refuse overlap (OOM risk; sole runner)" >&2
      exit 75
    fi
  else
    echo "G24: reclaiming empty/stale lock dir"
    rm -rf "$G24_LOCK"
    mkdir "$G24_LOCK"
  fi
fi
echo "pid=$$ host=$(hostname) ts=$(date -Iseconds)" >"$G24_LOCK/owner"
trap 'rm -rf "$G24_LOCK" 2>/dev/null || true' EXIT

if ! command -v git >/dev/null; then
  echo "G24: stock git missing in image" >&2
  exit 1
fi
echo "G24: stock git $(git --version)"
echo "G24: rustc $(rustc --version)"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/cargo-target}"

DC_ROOT="$(cd "$ROOT/.." && pwd)/discrete-circuit"
echo "G24: expecting discrete-circuit at $DC_ROOT"
if [[ ! -d "$DC_ROOT/packages/celir-index" || ! -d "$DC_ROOT/crates/celir-index-runtime" || ! -d "$DC_ROOT/packages/celir-data" ]]; then
  echo "G24: FAIL — discrete-circuit index packages missing (no fake PASS)" >&2
  exit 1
fi
echo "G24: discrete-circuit packages present"

echo "G24: default graph must stay CELIR-free"
if cargo tree -p arks-store --edges normal 2>/dev/null | grep -Ei 'celir' >/dev/null; then
  echo "G24: FAIL — default arks-store tree has celir edge" >&2
  exit 1
fi

mkdir -p verify/gates/g024-l3-celir-joint/out
export ARKS_G24_RECEIPT_OUT="${ARKS_G24_RECEIPT_OUT:-$ROOT/verify/gates/g024-l3-celir-joint/out/receipt.json}"
export ARKS_G24_DC_ROOT="$DC_ROOT"

echo "G24: joint suite (arks-lease --features celir-index-stress)"
cargo test -p arks-lease --features celir-index-stress --test g24_l3_celir_joint "$@"

echo "G24: writing receipt $ARKS_G24_RECEIPT_OUT"
python3 - <<'PY'
import json, os, subprocess, time
receipt = {
  "gate": "G24",
  "name": "l3-celir-index-joint",
  "result": "PASS",
  "n_tiers": [32, 128],
  "shared_index_n_way": "OUT",
  "stock_git": subprocess.check_output(["git", "--version"], text=True).strip(),
  "rustc": subprocess.check_output(["rustc", "--version"], text=True).strip(),
  "ts": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
  "feature": "celir-index-stress",
  "discrete_circuit": os.environ.get("ARKS_G24_DC_ROOT", ""),
}
path = os.environ["ARKS_G24_RECEIPT_OUT"]
with open(path, "w", encoding="utf-8") as f:
    json.dump(receipt, f, indent=2)
    f.write("\n")
print("G24: receipt written", path)
PY

echo "G24: PASS"
