#!/usr/bin/env bash
# L8 product-completion clean-room e2e — sequential Docker gates only.
# Matrix: verify/matrices/matrix-l8-e2e.md
# Host-only cargo is not Verified. Halt on failure / OOM; never invent PASS.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

COMPOSE=(docker compose -f verify/docker-compose.yml)
# Required core; expand list only when matrix-l8-e2e.md says so.
GATES=(g14 g16)

# Host Tee/redirects on Windows fail with exit 4294967295 if out/ is missing.
mkdir -p verify/gates/g14/out verify/gates/g16/out

echo "L8 e2e: sequential clean-room gates: ${GATES[*]}"
echo "L8 e2e: CARGO_BUILD_JOBS stays compose default (1); one gate at a time"

fail=0
pass=0
for g in "${GATES[@]}"; do
  echo "===== L8 e2e: starting ${g} ====="
  if "${COMPOSE[@]}" run --rm "$g"; then
    echo "===== L8 e2e: ${g} PASS ====="
    pass=$((pass + 1))
  else
    ec=$?
    echo "===== L8 e2e: ${g} FAIL (exit ${ec}) =====" >&2
    fail=$((fail + 1))
    echo "L8 e2e: halt (no stampede). pass=${pass} fail=${fail}" >&2
    exit "$ec"
  fi
done

echo "L8 e2e: ALL REQUIRED PASS (${pass}/${#GATES[@]})"
echo "L8 e2e: next expand per matrix-l8-e2e.md (g20/g21) when ready"
