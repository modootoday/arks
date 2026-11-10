#!/usr/bin/env bash
# FS-007 / WIN-003 host advisory gate — **trimmed**.
#
# `arks host-env` / `arks-hostfs` product surface was removed (not stock git).
# This script records an honest SKIP receipt and exits 0 so historical CI wiring
# does not fail closed on a deleted product CLI.
#
# Cite: .spec/research/20261111034556-overdesign-candidates-exhaustive.research.md (C13/C14)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUT_DIR="${ARKS_FS007_OUT:-${ROOT}/verify/gates/fs007-host-advisory/out}"
mkdir -p "${OUT_DIR}"
RECEIPT="${OUT_DIR}/receipt.json"

python3 - "$RECEIPT" <<'PY'
import json, sys, time
out = sys.argv[1]
receipt = {
    "gate": "fs007-host-advisory",
    "status": "SKIP",
    "reason": "arks host-env / arks-hostfs trimmed from product CLI (not git/git-daemon)",
    "ts": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
}
open(out, "w", encoding="utf-8").write(json.dumps(receipt, indent=2) + "\n")
print("fs007-host-advisory: SKIP — host-env trimmed from product CLI")
PY
