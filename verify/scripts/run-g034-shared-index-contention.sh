#!/usr/bin/env bash
# G34 — shared-index N-way contention clean-room (M26).
# SoT: shared-index-nway-clean-room; matrix: verify/matrices/matrix-g034-shared-index-contention.md
# Gate ≠ G22/G24; G33 reserved elsewhere.
set -euo pipefail

ROOT="${ARKS_G34_ROOT:-/workspace}"
cd "$ROOT"

mkdir -p verify/gates/g034-shared-index-contention
G34_LOCK="verify/gates/g034-shared-index-contention/.run.lock.dir"
if ! mkdir "$G34_LOCK" 2>/dev/null; then
  if [[ -f "$G34_LOCK/owner" ]]; then
    age=$(( $(date +%s) - $(stat -c %Y "$G34_LOCK/owner" 2>/dev/null || echo 0) ))
    if [[ "$age" -gt 7200 ]]; then
      echo "G34: reclaiming stale lock (age=${age}s)"
      rm -rf "$G34_LOCK"
      mkdir "$G34_LOCK"
    else
      echo "G34: another run holds $G34_LOCK — refuse overlap (sole runner)" >&2
      exit 75
    fi
  else
    echo "G34: reclaiming empty/stale lock dir"
    rm -rf "$G34_LOCK"
    mkdir "$G34_LOCK"
  fi
fi
echo "pid=$$ host=$(hostname) ts=$(date -Iseconds)" >"$G34_LOCK/owner"
trap 'rm -rf "$G34_LOCK" 2>/dev/null || true' EXIT

if ! command -v git >/dev/null; then
  echo "G34: stock git missing in image" >&2
  exit 1
fi
echo "G34: stock git $(git --version)"
echo "G34: rustc $(rustc --version)"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/cargo-target}"

echo "G34: gix-exit scan (workspace members)"
if cargo metadata --format-version 1 --no-deps 2>/dev/null | grep -E '"name":\s*"(gix|gitoxide)' >/dev/null; then
  echo "G34: forbidden gix/gitoxide package name in workspace members" >&2
  exit 1
fi

mkdir -p verify/gates/g034-shared-index-contention/out
export ARKS_G34_RECEIPT_OUT="${ARKS_G34_RECEIPT_OUT:-$ROOT/verify/gates/g034-shared-index-contention/out/receipt.json}"

echo "G34: shared-index contention suite (arks-lease --test g034-shared-index-contention_shared_index_contention)"
cargo test -p arks-lease --test g034-shared-index-contention_shared_index_contention "$@"

echo "G34: writing receipt $ARKS_G34_RECEIPT_OUT"
python3 - <<'PY'
import json, os, subprocess, time
receipt = {
  "gate": "G34",
  "name": "shared-index-contention",
  "result": "PASS",
  "n_floor": 2,
  "stock_git": subprocess.check_output(["git", "--version"], text=True).strip(),
  "rustc": subprocess.check_output(["rustc", "--version"], text=True).strip(),
  "ts": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
  "milestone": "M26",
  "not_substitute": ["M24", "G22", "G24", "G33"],
}
path = os.environ["ARKS_G34_RECEIPT_OUT"]
with open(path, "w", encoding="utf-8") as f:
    json.dump(receipt, f, indent=2)
    f.write("\n")
print("G34: receipt written", path)
PY

echo "G34: PASS"
