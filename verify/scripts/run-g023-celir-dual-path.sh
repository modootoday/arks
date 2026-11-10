#!/usr/bin/env bash
# G23 — CELIR dual-path Git verified clean-room (M24 successor).
# SoT: celir-dual-path-git-verified; ADR: 20261109204916; matrix: verify/matrices/matrix-g23.md
set -euo pipefail

ROOT="${ARKS_G23_ROOT:-/workspaces/gir}"
cd "$ROOT"

mkdir -p verify/gates/g23
G23_LOCK="verify/gates/g23/.run.lock.dir"
if ! mkdir "$G23_LOCK" 2>/dev/null; then
  if [[ -f "$G23_LOCK/owner" ]]; then
    age=$(( $(date +%s) - $(stat -c %Y "$G23_LOCK/owner" 2>/dev/null || echo 0) ))
    if [[ "$age" -gt 7200 ]]; then
      echo "G23: reclaiming stale lock (age=${age}s)"
      rm -rf "$G23_LOCK"
      mkdir "$G23_LOCK"
    else
      echo "G23: another run holds $G23_LOCK — refuse overlap (OOM risk; sole runner)" >&2
      exit 75
    fi
  else
    echo "G23: reclaiming empty/stale lock dir"
    rm -rf "$G23_LOCK"
    mkdir "$G23_LOCK"
  fi
fi
echo "pid=$$ host=$(hostname) ts=$(date -Iseconds)" >"$G23_LOCK/owner"
trap 'rm -rf "$G23_LOCK" 2>/dev/null || true' EXIT

if ! command -v git >/dev/null; then
  echo "G23: stock git missing in image" >&2
  exit 1
fi
echo "G23: stock git $(git --version)"
echo "G23: rustc $(rustc --version)"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/cargo-target}"
echo "G23: CARGO_BUILD_JOBS=$CARGO_BUILD_JOBS CARGO_TARGET_DIR=$CARGO_TARGET_DIR"

DC_ROOT="$(cd "$ROOT/.." && pwd)/discrete-circuit"
echo "G23: expecting discrete-circuit at $DC_ROOT"
if [[ ! -d "$DC_ROOT/packages/celir-index" || ! -d "$DC_ROOT/crates/celir-index-runtime" || ! -d "$DC_ROOT/packages/celir-data" ]]; then
  echo "G23: FAIL — discrete-circuit index packages missing (no fake PASS)" >&2
  exit 1
fi
echo "G23: discrete-circuit packages present"

echo "G23: default graph must stay CELIR-free"
if cargo tree -p arks-store --edges normal 2>/dev/null | grep -Ei 'celir' >/dev/null; then
  echo "G23: FAIL — default arks-store tree has celir edge" >&2
  exit 1
fi
echo "G23: default arks-store tree CELIR-free OK"

echo "G23: build arks binary (porcelain peer)"
cargo build -p arks
ARKS_BIN="$(find "$CARGO_TARGET_DIR" -name arks -type f 2>/dev/null | head -n1 || true)"
if [[ -z "$ARKS_BIN" || ! -f "$ARKS_BIN" ]]; then
  echo "G23: FAIL — arks binary missing after build" >&2
  exit 1
fi
export ARKS_BIN
echo "G23: ARKS_BIN=$ARKS_BIN"

mkdir -p verify/gates/g23/out
export ARKS_G23_RECEIPT_OUT="${ARKS_G23_RECEIPT_OUT:-$ROOT/verify/gates/g23/out/receipt.json}"
export ARKS_G23_DC_ROOT="$DC_ROOT"

echo "G23: dual-path suite (arks-lease --features celir-index-stress)"
cargo test -p arks-lease --features celir-index-stress --test g23_dual_path_git "$@"

echo "G23: writing receipt $ARKS_G23_RECEIPT_OUT"
python3 - <<'PY'
import json, os, subprocess, time
receipt = {
  "gate": "G23",
  "name": "celir-dual-path-git",
  "result": "PASS",
  "claim": "CELIR dual-path Git verified",
  "object_store": "filesystem-.git",
  "celir_role": "index-sidecar-only",
  "stock_git": subprocess.check_output(["git", "--version"], text=True).strip(),
  "rustc": subprocess.check_output(["rustc", "--version"], text=True).strip(),
  "ts": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
  "feature": "celir-index-stress",
  "discrete_circuit": os.environ.get("ARKS_G23_DC_ROOT", ""),
  "arks_bin": os.environ.get("ARKS_BIN", ""),
}
path = os.environ["ARKS_G23_RECEIPT_OUT"]
with open(path, "w", encoding="utf-8") as f:
    json.dump(receipt, f, indent=2)
    f.write("\n")
print("G23: receipt written", path)
PY

echo "G23: PASS"
