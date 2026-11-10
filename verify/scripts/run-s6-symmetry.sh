#!/usr/bin/env bash
# S6 expanded symmetry clean-room e2e — sequential Docker gates only.
# Matrix: verify/matrices/matrix-s6-symmetry.md
# Host-only cargo/gpg is not Verified. Halt on failure / OOM; never invent PASS.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

COMPOSE=(docker compose -f verify/docker-compose.yml)
GATES=(g38 g39 g40)

if [[ "${ARKS_S6_INCLUDE_G35:-0}" == "1" ]]; then
  GATES+=(g35)
fi
if [[ "${ARKS_S6_INCLUDE_L8:-0}" == "1" ]]; then
  GATES=(g14 g16 g20 g21 "${GATES[@]}")
fi

# Host Tee/redirects on Windows fail with exit 4294967295 if out/ is missing.
mkdir -p \
  verify/gates/g14/out verify/gates/g16/out verify/gates/g20/out verify/gates/g21/out \
  verify/gates/g35/out verify/gates/g38/out verify/gates/g39/out verify/gates/g40/out

echo "S6 symmetry: sequential clean-room gates: ${GATES[*]}"
echo "S6 symmetry: L8 cited Done unless ARKS_S6_INCLUDE_L8=1; CARGO_BUILD_JOBS=compose default"

fail=0
pass=0
declare -a RESULTS=()
for g in "${GATES[@]}"; do
  echo "===== S6: starting ${g} ====="
  if "${COMPOSE[@]}" run --rm --no-deps "$g"; then
    echo "===== S6: ${g} PASS ====="
    pass=$((pass + 1))
    RESULTS+=("${g}=PASS")
  else
    ec=$?
    echo "===== S6: ${g} FAIL (exit ${ec}) =====" >&2
    fail=$((fail + 1))
    RESULTS+=("${g}=FAIL")
    echo "S6: halt (no stampede). pass=${pass} fail=${fail}" >&2
    printf '%s\n' "${RESULTS[@]}" > verify/gates/g40/out/s6-summary.txt || true
    exit "$ec"
  fi
done

{
  echo "S6 symmetry: ALL REQUIRED PASS (${pass}/${#GATES[@]})"
  printf '%s\n' "${RESULTS[@]}"
  echo "S6: L8 historical Done cited (matrix-l8-e2e.md) unless re-run via ARKS_S6_INCLUDE_L8=1"
} | tee verify/gates/g40/out/s6-summary.txt

echo "S6 symmetry: PASS"
