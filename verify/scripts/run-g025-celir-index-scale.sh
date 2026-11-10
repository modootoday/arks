#!/usr/bin/env bash
# G25 — CELIR index scale clean-room (N=256 required; 384/512 stretch under OOM policy).
# SoT: celir-index-scale-clean-room; matrix: verify/matrices/matrix-g025-celir-index-scale.md
set -euo pipefail

ROOT="${ARKS_G25_ROOT:-/workspaces/gir}"
cd "$ROOT"

mkdir -p verify/gates/g025-celir-index-scale
G25_LOCK="verify/gates/g025-celir-index-scale/.run.lock.dir"
if ! mkdir "$G25_LOCK" 2>/dev/null; then
  if [[ -f "$G25_LOCK/owner" ]]; then
    age=$(( $(date +%s) - $(stat -c %Y "$G25_LOCK/owner" 2>/dev/null || echo 0) ))
    if [[ "$age" -gt 7200 ]]; then
      echo "G25: reclaiming stale lock (age=${age}s)"
      rm -rf "$G25_LOCK"
      mkdir "$G25_LOCK"
    else
      echo "G25: another run holds $G25_LOCK — refuse overlap (OOM risk; sole runner)" >&2
      exit 75
    fi
  else
    echo "G25: reclaiming empty/stale lock dir"
    rm -rf "$G25_LOCK"
    mkdir "$G25_LOCK"
  fi
fi
echo "pid=$$ host=$(hostname) ts=$(date -Iseconds)" >"$G25_LOCK/owner"
trap 'rm -rf "$G25_LOCK" 2>/dev/null || true' EXIT

if ! command -v git >/dev/null; then
  echo "G25: stock git missing in image" >&2
  exit 1
fi
echo "G25: stock git $(git --version)"
echo "G25: rustc $(rustc --version)"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/cargo-target}"

DC_ROOT="$(cd "$ROOT/.." && pwd)/discrete-circuit"
echo "G25: expecting discrete-circuit at $DC_ROOT"
if [[ ! -d "$DC_ROOT/packages/celir-index" || ! -d "$DC_ROOT/crates/celir-index-runtime" || ! -d "$DC_ROOT/packages/celir-data" ]]; then
  echo "G25: FAIL — discrete-circuit index packages missing (no fake PASS)" >&2
  exit 1
fi

echo "G25: default graph must stay CELIR-free"
if cargo tree -p arks-store --edges normal 2>/dev/null | grep -Ei 'celir' >/dev/null; then
  echo "G25: FAIL — default arks-store tree has celir edge" >&2
  exit 1
fi

mkdir -p verify/gates/g025-celir-index-scale/out
export ARKS_G25_RECEIPT_OUT="${ARKS_G25_RECEIPT_OUT:-$ROOT/verify/gates/g025-celir-index-scale/out/receipt.json}"
export ARKS_G25_DC_ROOT="$DC_ROOT"
# Stretch tiers may be skipped via env if prior OOM; required N=256 always runs.
export ARKS_G25_STRETCH="${ARKS_G25_STRETCH:-1}"

echo "G25: scale suite (arks-lease --features celir-index-stress)"
cargo test -p arks-lease --features celir-index-stress --test g025-celir-index-scale_index_scale -- --nocapture "$@"

echo "G25: writing receipt $ARKS_G25_RECEIPT_OUT"
python3 - <<'PY'
import json, os, subprocess, time
n_pass = [256]
stretch = {}
for key, n in (("n384", 384), ("n512", 512)):
    path = os.path.join(os.environ.get("ARKS_G25_ROOT", "/workspaces/gir"), "verify/gates/g025-celir-index-scale/out", f"{key}.status")
    if os.path.isfile(path):
        stretch[str(n)] = open(path, encoding="utf-8").read().strip()
        if stretch[str(n)] == "PASS":
            n_pass.append(n)
receipt = {
  "gate": "G25",
  "name": "celir-index-scale",
  "result": "PASS",
  "n_required": 256,
  "n_tiers_pass": n_pass,
  "stretch": stretch,
  "stock_git": subprocess.check_output(["git", "--version"], text=True).strip(),
  "rustc": subprocess.check_output(["rustc", "--version"], text=True).strip(),
  "ts": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
  "feature": "celir-index-stress",
  "discrete_circuit": os.environ.get("ARKS_G25_DC_ROOT", ""),
}
path = os.environ["ARKS_G25_RECEIPT_OUT"]
with open(path, "w", encoding="utf-8") as f:
    json.dump(receipt, f, indent=2)
    f.write("\n")
print("G25: receipt written", path)
PY

echo "G25: PASS"
