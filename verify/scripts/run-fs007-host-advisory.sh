#!/usr/bin/env bash
# FS-007 / WIN-003 host advisory gate — detect-only; never Claimed full cloud-sync.
#
# Opt-in probe root: ARKS_FS007_PROBE_ROOT (e.g. a throwaway dir under G:\…).
# Without it, probes CWD / repo and records honest "none" or advisory hazard.
#
# Cite: .spec/research/20261110222000-remaining-gaps-ffi-external-unblock.research.md
#       .spec/evidence/20261110221919-fs007-gdrive-host-advisory
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "${ROOT}"

OUT_DIR="${ARKS_FS007_OUT:-${ROOT}/verify/gates/fs007-host-advisory/out}"
mkdir -p "${OUT_DIR}"
RECEIPT="${OUT_DIR}/receipt.json"

export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"
cargo build -p arks --release
ARKS_BIN="$(find "${CARGO_TARGET_DIR:-target}/release" target/release -name arks -type f 2>/dev/null | head -n1 || true)"
if [[ -z "${ARKS_BIN}" || ! -x "${ARKS_BIN}" ]]; then
  echo "fs007-host-advisory: arks binary missing" >&2
  exit 1
fi

PROBE_ROOT="${ARKS_FS007_PROBE_ROOT:-.}"
echo "fs007-host-advisory: probe root=${PROBE_ROOT}"
set +e
HOST_OUT="$("${ARKS_BIN}" host-env "${PROBE_ROOT}" 2>"${OUT_DIR}/host-env.err")"
HOST_EC=$?
set -e
printf '%s\n' "${HOST_OUT}" | tee "${OUT_DIR}/host-env.out"
cat "${OUT_DIR}/host-env.err" >>"${OUT_DIR}/host-env.out" || true

python3 - "$RECEIPT" "$PROBE_ROOT" "$HOST_EC" "${OUT_DIR}/host-env.out" <<'PY'
import json, sys, time, re
out, probe, ec, log_path = sys.argv[1:5]
text = open(log_path, encoding="utf-8", errors="replace").read()
kind = None
m = re.search(r"^cloud\.kind:\s+(\S+)", text, re.M)
if m and m.group(1) != "(none)":
    kind = m.group(1)
placeholder = bool(re.search(r"^onedrive\.placeholder:\s+true", text, re.M))
hazard = "advisory" if (kind or placeholder) else "none"
receipt = {
    "schema": "arks-fs007-host-advisory/v1",
    "gate": "FS007-host-advisory",
    "disposition": "advisory-host-probe",
    "claimed": False,
    "fs007_full_sync_claimed": False,
    "win003_hydration_claimed": False,
    "probe_root": probe,
    "host_env_exit": int(ec),
    "cloud_kind": kind,
    "onedrive_placeholder": placeholder,
    "hazard": hazard,
    "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
    "note": "Detect/warn only; multi-device sync races and OneDrive hydration remain unclaimed",
}
with open(out, "w", encoding="utf-8") as f:
    json.dump(receipt, f, indent=2)
    f.write("\n")
print(f"fs007-host-advisory: receipt -> {out} hazard={hazard} claimed=false")
PY

echo "fs007-host-advisory: OK (advisory; ≠ Claimed FS-007)"
