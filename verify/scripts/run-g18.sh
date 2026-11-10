#!/usr/bin/env bash
# G18 — concurrency issue-repro compare differentiate/lease (M13 Cd-1..Cd-3 + M20 Cd-4..Cd-8).
# SoT: .spec/sot/interop-test-matrix.sot.md (G18); matrix: verify/matrices/matrix-g18.md
# Done: every claimed ID has git pathology_pass + arks resolved_pass in Docker receipt.
set -euo pipefail

cd /workspace

if ! command -v git >/dev/null; then
  echo "G18: stock git missing in image" >&2
  exit 1
fi
echo "G18: stock git $(git --version)"
echo "G18: rustc $(rustc --version)"
echo "G18: CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-2}"

export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"

echo "G18: gix-exit / no-celir scan"
if cargo metadata --format-version 1 --no-deps 2>/dev/null | grep -E '"name":\s*"(gix|gitoxide)' >/dev/null; then
  echo "G18: forbidden gix/gitoxide package name in workspace members" >&2
  exit 1
fi
meta="$(cargo metadata --format-version 1)"
if echo "$meta" | grep -E '"name":\s*"gix|"name":\s*"gitoxide|"name":\s*"gix-' >/dev/null; then
  echo "G18: forbidden gix/gitoxide edge in cargo metadata" >&2
  exit 1
fi
if echo "$meta" | grep -Ei '"name":\s*"celir|"name":\s*"celir-' >/dev/null; then
  echo "G18: forbidden celir edge in cargo metadata" >&2
  exit 1
fi
echo "G18: gix-exit / no-celir OK"

echo "G18: build arks binary (release)"
cargo build -p arks --release

mkdir -p verify/gates/g18/out
export ARKS_G18_RECEIPT_OUT="${ARKS_G18_RECEIPT_OUT:-/workspace/verify/gates/g18/out/receipt.json}"
export ARKS_G18_VENUE="${ARKS_G18_VENUE:-clean-room}"
export ARKS_G18_IMAGE_TAG="${ARKS_G18_IMAGE_TAG:-arks-verify-m1:0.1.0}"
export ARKS_G18_REQUIRE_FULL=1
if [[ -z "${ARKS_G18_IMAGE_DIGEST:-}" && -f verify/images.lock.json ]]; then
  ARKS_G18_IMAGE_DIGEST="$(
    awk '
      /"role": "m1-cleanroom"/ { in_m1=1 }
      in_m1 && /"digest":/ {
        gsub(/[",]/, "", $2);
        gsub(/\r/, "", $2);
        print $2;
        exit
      }
    ' verify/images.lock.json
  )"
  ARKS_G18_IMAGE_DIGEST="${ARKS_G18_IMAGE_DIGEST//$'\r'/}"
fi
export ARKS_G18_IMAGE_DIGEST="${ARKS_G18_IMAGE_DIGEST:-}"
export ARKS_G18_REVISION="$(git -C /workspace rev-parse HEAD 2>/dev/null || echo unknown)"

echo "G18: issue-compare suite (verify/gates/g18; serial threads for receipt)"
# --test-threads=1 so RECEIPT static accumulates all rows before zzz flush.
cargo test --manifest-path verify/gates/g18/Cargo.toml -- --nocapture --test-threads=1 "$@"

if [[ ! -f "$ARKS_G18_RECEIPT_OUT" ]]; then
  echo "G18: receipt missing: $ARKS_G18_RECEIPT_OUT" >&2
  exit 1
fi
echo "G18: receipt written to $ARKS_G18_RECEIPT_OUT"
head -n 260 "$ARKS_G18_RECEIPT_OUT" || true

echo "G18: Cd-1..Cd-8 oracle gate (scenario IDs + dual PASS)"
for id in \
  'REF-001' 'REF-006' 'REF-011' 'AGENT-003' \
  'LOCK-002' 'WT-001' 'SUB-001' 'AGENT-007' \
  'HOOK-001/HOOK-006' 'PACK-004/MAINT-003' 'AGENT-002' 'IDX-001' \
  'AGENT-001' 'AGENT-010' 'AGENT-011' 'REF-002' 'REF-004' 'REF-008' \
  'REF-010' 'IDX-003' 'IDX-007' 'SEQ-005' 'WT-003' 'MAINT-001' \
  'AGENT-008' 'AGENT-009' 'AGENT-012' 'REF-005' 'REF-014' 'REF-015' \
  'IDX-009' 'IDX-012' 'WT-007' 'WT-008' 'CFG-001' 'TOCTOU-001' \
  'HOOK-004' 'CFG-003' 'REF-019' 'REF-023' 'OBJ-002' 'OBJ-003' \
  'SUB-002' 'SUB-004' 'SUB-007' 'TOCTOU-002' 'TOCTOU-006' 'MISC-004' \
  'MISC-010' 'MISC-015' 'MISC-016' 'SUB-010' 'SHAL-002' 'PACK-007' \
  'CI-003' 'CI-004' 'NET-001' 'NET-002' 'NET-007' 'LFS-002' \
  'PERF-001' 'PERF-002' 'PERF-003' 'PERF-004' 'PERF-005' 'PERF-006' \
  'PERF-007' 'PERF-008' 'PERF-009' 'PERF-010' 'CFG-002' 'AGENT-004'
do
  if ! grep -F "\"scenario_id\": \"$id\"" "$ARKS_G18_RECEIPT_OUT" >/dev/null; then
    echo "G18: missing scenario $id" >&2
    exit 1
  fi
done
pathology_n="$(grep -c '"git_outcome": "pathology_pass"' "$ARKS_G18_RECEIPT_OUT" || true)"
resolved_n="$(grep -c '"arks_outcome": "resolved_pass"' "$ARKS_G18_RECEIPT_OUT" || true)"
if [[ "$pathology_n" -lt 72 || "$resolved_n" -lt 72 ]]; then
  echo "G18: expected >=72 pathology_pass and >=72 resolved_pass; got $pathology_n / $resolved_n" >&2
  exit 1
fi
if grep -E '"git_outcome": "fail"|"arks_outcome": "fail"' "$ARKS_G18_RECEIPT_OUT" >/dev/null; then
  echo "G18: receipt contains fail outcomes" >&2
  exit 1
fi
echo "G18: Cd-8 oracle gate PASS (includes Cd-1..Cd-7 regression)"

echo "G18: PASS"
