#!/usr/bin/env bash
# G27 — arks smart HTTP + synthetic PR contention clean-room.
# SoT: arks-smart-http-pr-contention-clean-room; matrix: verify/matrix-g27.md
# Honesty: arks stand-in ≠ GitHub UX/Checks/merge-queue; ≠ live forge throughput.
# Sole-runner vs heavy gates.
set -euo pipefail

ROOT="${ARKS_G27_ROOT:-/workspace}"
if [[ -d /workspaces/gir/verify ]]; then
  ROOT="${ARKS_G27_ROOT:-/workspaces/gir}"
fi
cd "$ROOT"

mkdir -p verify/g27/out
G27_LOCK="verify/g27/.run.lock.dir"

# Refuse overlap with other heavy clean-room runners (OOM lesson).
for other in g17 g19 g22 g23 g24 g25 g26; do
  if [[ -d "verify/${other}/.run.lock.dir" ]]; then
    echo "G27: heavy gate ${other} holds verify/${other}/.run.lock.dir — refuse overlap" >&2
    exit 75
  fi
done

if ! mkdir "$G27_LOCK" 2>/dev/null; then
  if [[ -f "$G27_LOCK/owner" ]]; then
    age=$(( $(date +%s) - $(stat -c %Y "$G27_LOCK/owner" 2>/dev/null || echo 0) ))
    if [[ "$age" -gt 7200 ]]; then
      echo "G27: reclaiming stale lock (age=${age}s)"
      rm -rf "$G27_LOCK"
      mkdir "$G27_LOCK"
    else
      echo "G27: another run holds $G27_LOCK — refuse overlap (sole runner)" >&2
      exit 75
    fi
  else
    echo "G27: reclaiming empty/stale lock dir"
    rm -rf "$G27_LOCK"
    mkdir "$G27_LOCK"
  fi
fi
echo "pid=$$ host=$(hostname) ts=$(date -Iseconds)" >"$G27_LOCK/owner"
trap 'rm -rf "$G27_LOCK" 2>/dev/null || true' EXIT

if ! command -v git >/dev/null; then
  echo "G27: stock git missing in image" >&2
  exit 1
fi
echo "G27: stock git $(git --version)"
echo "G27: rustc $(rustc --version)"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/cargo-target}"
echo "G27: CARGO_BUILD_JOBS=$CARGO_BUILD_JOBS CARGO_TARGET_DIR=$CARGO_TARGET_DIR"

echo "G27: gix-exit scan"
if cargo metadata --format-version 1 --no-deps 2>/dev/null | grep -E '"name":\s*"(gix|gitoxide)' >/dev/null; then
  echo "G27: forbidden gix/gitoxide workspace member" >&2
  exit 1
fi

echo "G27: build arks binary (serve-http)"
cargo build -p arks

export ARKS_G27_RECEIPT_OUT="${ARKS_G27_RECEIPT_OUT:-$ROOT/verify/g27/out/receipt.json}"
export ARKS_G27_SCALE_OUT="${ARKS_G27_SCALE_OUT:-$ROOT/verify/g27/out/scale.json}"
# Defaults; tests may overwrite scale.json with actual achieved N.
export ARKS_G27_N_BRANCHES="${ARKS_G27_N_BRANCHES:-2048}"
export ARKS_G27_N_PR="${ARKS_G27_N_PR:-2048}"
export ARKS_G27_N_AGENTS="${ARKS_G27_N_AGENTS:-8}"

echo "G27: smart HTTP PR contention suite (verify/g27; serial — sole HTTP listeners)"
cargo test --manifest-path verify/g27/Cargo.toml -- --nocapture --test-threads=1 "$@"

# Prefer scale file written by tests when present.
if [[ -f "$ARKS_G27_SCALE_OUT" ]]; then
  echo "G27: scale file present $ARKS_G27_SCALE_OUT"
  # shellcheck disable=SC1091
  eval "$(python3 - <<'PY'
import json, os
p = os.environ["ARKS_G27_SCALE_OUT"]
with open(p, encoding="utf-8") as f:
    d = json.load(f)
for k, env in [("n_branches","ARKS_G27_N_BRANCHES"),("n_pr","ARKS_G27_N_PR"),("n_agents","ARKS_G27_N_AGENTS")]:
    if k in d:
        print(f'export {env}={int(d[k])}')
PY
)"
fi

echo "G27: writing receipt $ARKS_G27_RECEIPT_OUT"
python3 - <<'PY'
import json, os, subprocess, time
receipt = {
  "gate": "G27",
  "name": "arks-smart-http-pr-contention",
  "result": "PASS",
  "n_branches": int(os.environ["ARKS_G27_N_BRANCHES"]),
  "n_pr": int(os.environ["ARKS_G27_N_PR"]),
  "n_agents": int(os.environ["ARKS_G27_N_AGENTS"]),
  "stock_git": subprocess.check_output(["git", "--version"], text=True).strip(),
  "rustc": subprocess.check_output(["rustc", "--version"], text=True).strip(),
  "ts": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
  "honesty": "arks smart-HTTP stand-in != GitHub UX/Checks/merge-queue; != live forge throughput; forge Deferred G8+",
}
path = os.environ["ARKS_G27_RECEIPT_OUT"]
with open(path, "w", encoding="utf-8") as f:
    json.dump(receipt, f, indent=2)
    f.write("\n")
print("G27: receipt written", path)
print(json.dumps(receipt, indent=2))
PY

echo "G27: PASS"
