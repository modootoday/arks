#!/usr/bin/env bash
# FS-014 LSM advisory gate — **trimmed**.
#
# `arks host-env` product CLI removed; LSM detect lived behind that surface.
# Honest SKIP receipt; exit 0 for historical CI wiring.
#
# Cite: .spec/research/20261111034556-overdesign-candidates-exhaustive.research.md (C13/C14)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUT_DIR="${ARKS_FS014_OUT:-${ROOT}/verify/gates/fs014-lsm-advisory/out}"
mkdir -p "${OUT_DIR}"
RECEIPT="${OUT_DIR}/receipt.json"

python3 - "$RECEIPT" <<'PY'
import json, sys, time
out = sys.argv[1]
receipt = {
    "gate": "fs014-lsm-advisory",
    "status": "SKIP",
    "reason": "arks host-env / arks-hostfs trimmed from product CLI (not git/git-daemon)",
    "ts": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
}
open(out, "w", encoding="utf-8").write(json.dumps(receipt, indent=2) + "\n")
print("fs014-lsm-advisory: SKIP — host-env trimmed from product CLI")
PY
