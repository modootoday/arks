#!/usr/bin/env bash
# G22 — CELIR index contention clean-room (M24).
# SoT: celir-index-contention-clean-room; matrix: verify/matrix-g22.md
# discrete-circuit MUST be mounted as sibling of gir (see compose service g22).
set -euo pipefail

# Compose sets working_dir=/workspaces/gir for path-dep layout.
ROOT="${ARKS_G22_ROOT:-/workspaces/gir}"
cd "$ROOT"

# Exclusive lock: do not overlap heavy G17/G19 (OOM lesson).
mkdir -p verify/g22
G22_LOCK="verify/g22/.run.lock.dir"
if ! mkdir "$G22_LOCK" 2>/dev/null; then
  if [[ -f "$G22_LOCK/owner" ]]; then
    age=$(( $(date +%s) - $(stat -c %Y "$G22_LOCK/owner" 2>/dev/null || echo 0) ))
    if [[ "$age" -gt 7200 ]]; then
      echo "G22: reclaiming stale lock (age=${age}s)"
      rm -rf "$G22_LOCK"
      mkdir "$G22_LOCK"
    else
      echo "G22: another run holds $G22_LOCK — refuse overlap (OOM risk; sole runner)" >&2
      exit 75
    fi
  else
    echo "G22: reclaiming empty/stale lock dir"
    rm -rf "$G22_LOCK"
    mkdir "$G22_LOCK"
  fi
fi
echo "pid=$$ host=$(hostname) ts=$(date -Iseconds)" >"$G22_LOCK/owner"
trap 'rm -rf "$G22_LOCK" 2>/dev/null || true' EXIT

if ! command -v git >/dev/null; then
  echo "G22: stock git missing in image" >&2
  exit 1
fi
echo "G22: stock git $(git --version)"
echo "G22: rustc $(rustc --version)"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/cargo-target}"
echo "G22: CARGO_BUILD_JOBS=$CARGO_BUILD_JOBS CARGO_TARGET_DIR=$CARGO_TARGET_DIR"

# G22-C06 — discrete-circuit sibling mount for path-deps.
DC_ROOT="$(cd "$ROOT/.." && pwd)/discrete-circuit"
echo "G22: expecting discrete-circuit at $DC_ROOT"
if [[ ! -d "$DC_ROOT/packages/celir-index" || ! -d "$DC_ROOT/crates/celir-index-runtime" || ! -d "$DC_ROOT/packages/celir-data" ]]; then
  echo "G22: FAIL — discrete-circuit index packages missing (no fake PASS)" >&2
  echo "G22: mount host sibling discrete-circuit at $DC_ROOT" >&2
  exit 1
fi
echo "G22: discrete-circuit packages present"

echo "G22: default graph must stay CELIR-free"
if cargo tree -p arks-store --edges normal 2>/dev/null | grep -Ei 'celir' >/dev/null; then
  echo "G22: FAIL — default arks-store tree has celir edge" >&2
  cargo tree -p arks-store --edges normal >&2 || true
  exit 1
fi
echo "G22: default arks-store tree CELIR-free OK"

echo "G22: gix-exit scan (workspace members)"
if cargo metadata --format-version 1 --no-deps 2>/dev/null | grep -E '"name":\s*"(gix|gitoxide)' >/dev/null; then
  echo "G22: forbidden gix/gitoxide package name in workspace members" >&2
  exit 1
fi

mkdir -p verify/g22/out
export ARKS_G22_RECEIPT_OUT="${ARKS_G22_RECEIPT_OUT:-$ROOT/verify/g22/out/receipt.json}"
export ARKS_G22_DC_ROOT="$DC_ROOT"

echo "G22: stress suite (arks-lease --features celir-index-stress)"
cargo test -p arks-lease --features celir-index-stress --test g22_index_contention "$@"

echo "G22: arks-store feature-on regress"
cargo test -p arks-store --features celir-index "$@"

echo "G22: writing receipt $ARKS_G22_RECEIPT_OUT"
python3 - <<'PY'
import json, os, subprocess, time
receipt = {
  "gate": "G22",
  "name": "celir-index-contention",
  "result": "PASS",
  "n_tiers": [32, 128],
  "stock_git": subprocess.check_output(["git", "--version"], text=True).strip(),
  "rustc": subprocess.check_output(["rustc", "--version"], text=True).strip(),
  "ts": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
  "feature": "celir-index-stress",
  "discrete_circuit": os.environ.get("ARKS_G22_DC_ROOT", ""),
}
path = os.environ["ARKS_G22_RECEIPT_OUT"]
with open(path, "w", encoding="utf-8") as f:
    json.dump(receipt, f, indent=2)
    f.write("\n")
print("G22: receipt written", path)
PY

echo "G22: PASS"
