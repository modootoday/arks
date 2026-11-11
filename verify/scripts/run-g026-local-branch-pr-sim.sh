#!/usr/bin/env bash
# G26 — local branch + synthetic PR-env clean-room (stream B).
# SoT: local-branch-pr-sim-clean-room; matrix: verify/matrices/matrix-g026-local-branch-pr-sim.md
# Honesty: local sim ≠ GitHub PR throughput. Sole-runner vs heavy gates.
set -euo pipefail

ROOT="${ARKS_G26_ROOT:-/workspace}"
# Prefer /workspaces/gir when compose uses that working_dir.
if [[ -d /workspaces/gir/verify ]]; then
  ROOT="${ARKS_G26_ROOT:-/workspaces/gir}"
fi
cd "$ROOT"

mkdir -p verify/gates/g026-local-branch-pr-sim/out
G26_LOCK="verify/gates/g026-local-branch-pr-sim/.run.lock.dir"

# Refuse overlap with other heavy clean-room runners (OOM lesson).
for other in g017-concurrency-issue-compare g019-comparative-performance g022-celir-index-contention g023-celir-dual-path g024-l3-celir-joint g025-celir-index-scale g027-smart-http-pr-contention; do
  if [[ -d "verify/${other}/.run.lock.dir" ]]; then
    echo "G26: heavy gate ${other} holds verify/${other}/.run.lock.dir — refuse overlap" >&2
    exit 75
  fi
done

if ! mkdir "$G26_LOCK" 2>/dev/null; then
  if [[ -f "$G26_LOCK/owner" ]]; then
    age=$(( $(date +%s) - $(stat -c %Y "$G26_LOCK/owner" 2>/dev/null || echo 0) ))
    if [[ "$age" -gt 7200 ]]; then
      echo "G26: reclaiming stale lock (age=${age}s)"
      rm -rf "$G26_LOCK"
      mkdir "$G26_LOCK"
    else
      echo "G26: another run holds $G26_LOCK — refuse overlap (sole runner)" >&2
      exit 75
    fi
  else
    echo "G26: reclaiming empty/stale lock dir"
    rm -rf "$G26_LOCK"
    mkdir "$G26_LOCK"
  fi
fi
echo "pid=$$ host=$(hostname) ts=$(date -Iseconds)" >"$G26_LOCK/owner"
trap 'rm -rf "$G26_LOCK" 2>/dev/null || true' EXIT

if ! command -v git >/dev/null; then
  echo "G26: stock git missing in image" >&2
  exit 1
fi
echo "G26: stock git $(git --version)"
echo "G26: rustc $(rustc --version)"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/cargo-target}"
echo "G26: CARGO_BUILD_JOBS=$CARGO_BUILD_JOBS CARGO_TARGET_DIR=$CARGO_TARGET_DIR"

echo "G26: gix-exit scan"
if cargo metadata --format-version 1 --no-deps 2>/dev/null | grep -E '"name":\s*"(gix|gitoxide)' >/dev/null; then
  echo "G26: forbidden gix/gitoxide workspace member" >&2
  exit 1
fi

export ARKS_G26_RECEIPT_OUT="${ARKS_G26_RECEIPT_OUT:-$ROOT/verify/gates/g026-local-branch-pr-sim/out/receipt.json}"
export ARKS_G26_SCALE_OUT="${ARKS_G26_SCALE_OUT:-$ROOT/verify/gates/g026-local-branch-pr-sim/out/scale.json}"

echo "G26: branch/PR sim suite (arks-lease --test g26_branch_pr_sim)"
cargo test -p arks-lease --test g26_branch_pr_sim -- --nocapture "$@"

# Documented actual N (tests pin these minima).
export ARKS_G26_N_BRANCHES="${ARKS_G26_N_BRANCHES:-2048}"
export ARKS_G26_N_PR="${ARKS_G26_N_PR:-2048}"
export ARKS_G26_N_WORKERS="${ARKS_G26_N_WORKERS:-32}"
if [[ -f "$ARKS_G26_SCALE_OUT" ]]; then
  echo "G26: scale file present $ARKS_G26_SCALE_OUT"
fi

echo "G26: writing receipt $ARKS_G26_RECEIPT_OUT"
python3 - <<'PY'
import json, os, subprocess, time
receipt = {
  "gate": "G26",
  "name": "local-branch-pr-sim",
  "result": "PASS",
  "n_branches": int(os.environ["ARKS_G26_N_BRANCHES"]),
  "n_pr": int(os.environ["ARKS_G26_N_PR"]),
  "n_workers": int(os.environ["ARKS_G26_N_WORKERS"]),
  "stock_git": subprocess.check_output(["git", "--version"], text=True).strip(),
  "rustc": subprocess.check_output(["rustc", "--version"], text=True).strip(),
  "ts": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
  "honesty": "local sim != GitHub PR throughput; forge Deferred G8+",
}
path = os.environ["ARKS_G26_RECEIPT_OUT"]
with open(path, "w", encoding="utf-8") as f:
    json.dump(receipt, f, indent=2)
    f.write("\n")
print("G26: receipt written", path)
print(json.dumps(receipt, indent=2))
PY

echo "G26: PASS"
