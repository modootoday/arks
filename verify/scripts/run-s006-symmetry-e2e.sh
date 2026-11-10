#!/usr/bin/env bash
# S6 expanded symmetry clean-room e2e — sequential Docker gates only.
# Matrix: verify/matrices/matrix-s006-symmetry-e2e.md
# Host-only cargo/gpg is not Verified. Halt on failure / OOM; never invent PASS.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

COMPOSE=(docker compose -f verify/docker-compose.yml)
GATES=(g038-gpgsig-openpgp g039-partial-clone-filter g040-dumb-http-push-deletes)

if [[ "${ARKS_S6_INCLUDE_G35:-0}" == "1" ]]; then
  GATES+=(g035-shallow-depth)
fi
if [[ "${ARKS_S6_INCLUDE_L8:-0}" == "1" ]]; then
  GATES=(g014-cli-parity-production g016-journey-compare g020-cli-symmetric-parity g021-cli-flag-symmetry "${GATES[@]}")
fi

# Host Tee/redirects on Windows fail with exit 4294967295 if out/ is missing.
mkdir -p \
  verify/gates/g014-cli-parity-production/out verify/gates/g016-journey-compare/out verify/gates/g020-cli-symmetric-parity/out verify/gates/g021-cli-flag-symmetry/out \
  verify/gates/g035-shallow-depth/out verify/gates/g038-gpgsig-openpgp/out verify/gates/g039-partial-clone-filter/out verify/gates/g040-dumb-http-push-deletes/out

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
    printf '%s\n' "${RESULTS[@]}" > verify/gates/g040-dumb-http-push-deletes/out/s6-summary.txt || true
    exit "$ec"
  fi
done

{
  echo "S6 symmetry: ALL REQUIRED PASS (${pass}/${#GATES[@]})"
  printf '%s\n' "${RESULTS[@]}"
  echo "S6: L8 historical Done cited (matrix-l008-product-completion-e2e.md) unless re-run via ARKS_S6_INCLUDE_L8=1"
} | tee verify/gates/g040-dumb-http-push-deletes/out/s6-summary.txt

echo "S6 symmetry: PASS"
