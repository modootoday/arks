#!/usr/bin/env bash
# FS-014 LSM detect + errno honesty — advisory; never Claimed host-policy bypass.
#
# Probes /sys/fs/selinux/enforce and AppArmor sysfs; runs `arks host-env` and
# records LSM fields. Optional Docker AppArmor profile (ARKS_FS014_APPARMOR=1)
# only documents confinement presence — does not invent bypass Claimed.
#
# Cite: .spec/research/20261111001816-partial-completion-unblock-roadmap.research.md
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "${ROOT}"

OUT_DIR="${ARKS_FS014_OUT:-${ROOT}/verify/gates/fs014-lsm-advisory/out}"
mkdir -p "${OUT_DIR}"
RECEIPT="${OUT_DIR}/receipt.json"

export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"
cargo build -p arks --release
ARKS_BIN="$(find "${CARGO_TARGET_DIR:-target}/release" target/release -name arks -type f 2>/dev/null | head -n1 || true)"
if [[ -z "${ARKS_BIN}" || ! -x "${ARKS_BIN}" ]]; then
  echo "fs014-lsm-advisory: arks binary missing" >&2
  exit 1
fi

PROBE_ROOT="${ARKS_FS014_PROBE_ROOT:-.}"
echo "fs014-lsm-advisory: probe root=${PROBE_ROOT}"

SELINUX="absent"
if [[ -r /sys/fs/selinux/enforce ]]; then
  SELINUX="$(tr -d '\n' </sys/fs/selinux/enforce)"
fi
APPARMOR="absent"
if [[ -r /sys/module/apparmor/parameters/enabled ]]; then
  APPARMOR="$(tr -d '\n' </sys/module/apparmor/parameters/enabled)"
fi

set +e
HOST_OUT="$("${ARKS_BIN}" host-env "${PROBE_ROOT}" 2>"${OUT_DIR}/host-env.err")"
HOST_EC=$?
set -e
printf '%s\n' "${HOST_OUT}" | tee "${OUT_DIR}/host-env.out"
cat "${OUT_DIR}/host-env.err" >>"${OUT_DIR}/host-env.out" || true

python3 - "$RECEIPT" "$PROBE_ROOT" "$HOST_EC" "${OUT_DIR}/host-env.out" "$SELINUX" "$APPARMOR" <<'PY'
import json, sys, time, re
out, probe, ec, log_path, selinux, apparmor = sys.argv[1:7]
text = open(log_path, encoding="utf-8", errors="replace").read()
lsm = None
m = re.search(r"^lsm\.kind:\s+(\S+)", text, re.M)
if m and m.group(1) != "(none)":
    lsm = m.group(1)
receipt = {
    "schema": "arks-fs014-lsm-advisory/v1",
    "gate": "FS014-lsm-advisory",
    "disposition": "advisory-lsm-detect",
    "claimed": False,
    "fs014_lsm_bypass_claimed": False,
    "probe_root": probe,
    "host_env_exit": int(ec),
    "sysfs_selinux_enforce": selinux,
    "sysfs_apparmor_enabled": apparmor,
    "lsm_kind": lsm,
    "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
    "note": "Detect + errno honesty only; host LSM policy cannot be implemented/bypassed inside arks",
}
with open(out, "w", encoding="utf-8") as f:
    json.dump(receipt, f, indent=2)
    f.write("\n")
print(f"fs014-lsm-advisory: receipt -> {out} lsm={lsm} claimed=false")
PY

echo "fs014-lsm-advisory: OK (advisory; ≠ Claimed FS-014 bypass)"
