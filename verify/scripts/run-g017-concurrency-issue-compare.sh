#!/usr/bin/env bash
# G17 — concurrency issue-repro compare (M12 Cm + M19 Cx + M20 Cx-3..Cx-8 + M21 Dz-1).
# SoT: .spec/sot/interop-test-matrix.sot.md (G17); matrix: verify/matrices/matrix-g017-concurrency-issue-compare.md
# Done: every claimed ID has git pathology_pass + arks resolved_pass in Docker receipt.
set -euo pipefail

cd /workspace

if ! command -v git >/dev/null; then
  echo "G17: stock git missing in image" >&2
  exit 1
fi
echo "G17: stock git $(git --version)"
echo "G17: rustc $(rustc --version)"
echo "G17: CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-2}"

export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"

echo "G17: gix-exit scan"
if cargo metadata --format-version 1 --no-deps 2>/dev/null | grep -E '"name":\s*"(gix|gitoxide)' >/dev/null; then
  echo "G17: forbidden gix/gitoxide package name in workspace members" >&2
  exit 1
fi
meta="$(cargo metadata --format-version 1)"
if echo "$meta" | grep -E '"name":\s*"gix|"name":\s*"gitoxide|"name":\s*"gix-' >/dev/null; then
  echo "G17: forbidden gix/gitoxide edge in cargo metadata" >&2
  exit 1
fi
echo "G17: gix-exit OK"

echo "G17: build arks binary (release)"
cargo build -p arks --release

mkdir -p verify/gates/g017-concurrency-issue-compare/out
export ARKS_G17_RECEIPT_OUT="${ARKS_G17_RECEIPT_OUT:-/workspace/verify/gates/g017-concurrency-issue-compare/out/receipt.json}"
export ARKS_G17_VENUE="${ARKS_G17_VENUE:-clean-room}"
export ARKS_G17_IMAGE_TAG="${ARKS_G17_IMAGE_TAG:-arks-verify-m1:0.1.0}"
export ARKS_G17_REQUIRE_FULL=1
if [[ -z "${ARKS_G17_IMAGE_DIGEST:-}" && -f verify/images.lock.json ]]; then
  ARKS_G17_IMAGE_DIGEST="$(
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
  ARKS_G17_IMAGE_DIGEST="${ARKS_G17_IMAGE_DIGEST//$'\r'/}"
fi
export ARKS_G17_IMAGE_DIGEST="${ARKS_G17_IMAGE_DIGEST:-}"
export ARKS_G17_REVISION="$(git -C /workspace rev-parse HEAD 2>/dev/null || echo unknown)"

echo "G17: issue-compare suite (verify/gates/g017-concurrency-issue-compare; serial threads for receipt)"
# --test-threads=1 so RECEIPT static accumulates all rows before zzz flush.
cargo test --manifest-path verify/gates/g017-concurrency-issue-compare/Cargo.toml -- --nocapture --test-threads=1 "$@"

if [[ ! -f "$ARKS_G17_RECEIPT_OUT" ]]; then
  echo "G17: receipt missing: $ARKS_G17_RECEIPT_OUT" >&2
  exit 1
fi
echo "G17: receipt written to $ARKS_G17_RECEIPT_OUT"
head -n 200 "$ARKS_G17_RECEIPT_OUT" || true

echo "G17: Cm-1..Cm-3 + Cx-1..Cx-8 oracle gate (scenario IDs + dual PASS)"
for id in \
  'AGENT-006/IDX-008' 'LOCK-002' 'WT-001' 'SUB-001' \
  'IDX-001' 'HOOK-001/HOOK-006' 'PACK-004/MAINT-003' 'AGENT-003' \
  'IDX-002' 'AGENT-005' 'AGENT-007' 'CORR-001' 'FS-006' \
  'REF-007' 'LOCK-001' 'WT-002' 'WT-004' \
  'SEQ-001' 'SEQ-002' 'SEQ-003' 'REF-018' 'MISC-012' 'TOCTOU-005' \
  'BISECT-001' 'BISECT-002' 'SEQ-006' 'SEQ-007' 'MISC-007' \
  'STASH-001' 'STASH-002' 'REF-021' 'TOCTOU-003' 'MISC-005' \
  'CORR-002' 'LOCK-004' \
  'REF-012' 'REF-016' 'IDX-014' 'SPARSE-001' 'MISC-018' 'SUB-008' \
  'TOCTOU-007' 'PACK-009' 'MAINT-002' 'NET-009' 'CFG-004' 'MISC-017' \
  'REF-009' 'REF-022' 'IDX-013' 'IDX-010' 'HOOK-002' 'HOOK-005' \
  'MISC-014' 'MISC-011' 'SHAL-001' 'WT-006' 'SUB-006' 'PACK-008' \
  'IDX-004' 'IDX-011' 'HOOK-003' 'MISC-019' 'OBJ-001' 'OBJ-005' \
  'PACK-001' 'WT-005' 'LOCK-003' 'MISC-002' 'MISC-003' 'SHAL-003' \
  'OBJ-008' 'PACK-002' 'PACK-003' 'WT-009' 'WT-010' 'SUB-003' \
  'SUB-009' 'SEQ-004' 'CORR-004' 'CORR-005' 'LOCK-007' 'MISC-020' \
  'FSM-001' 'FSM-002' 'FSM-004' 'IDX-005' 'MISC-008' 'SHAL-004' \
  'SPARSE-002' 'PACK-005' 'MAINT-005' 'CORR-003' 'CI-001' 'FS-008' \
  'CI-005' 'IDX-006' 'FSM-003' 'REF-003' 'MAINT-006' 'MISC-001' \
  'MISC-009' 'OBJ-006' 'CORR-006' 'CORR-007' 'NET-010' 'PACK-006' \
  'FS-001' 'FS-002' 'FS-003' 'FS-004' 'FS-005' 'FS-009' \
  'FS-010' 'FS-011' 'FS-012' 'FS-013' 'WIN-001' 'WIN-002' \
  'LOCK-005' 'LOCK-006' 'LOCK-008' 'LOCK-009' 'REF-013' 'REF-020' \
  'OBJ-007' 'OBJ-009' 'SUB-005' 'MISC-013' 'HOOK-007' 'CI-002' \
  'CRED-001' 'CRED-002' 'NET-003' 'NET-004' 'NET-005' 'NET-008' 'NET-011' \
  'LFS-001' 'LFS-003' 'LFS-004' 'LFS-005' 'LFS-006' \
  'CI-006' 'TOCTOU-004'
do
  if ! grep -F "\"scenario_id\": \"$id\"" "$ARKS_G17_RECEIPT_OUT" >/dev/null; then
    echo "G17: missing scenario $id" >&2
    exit 1
  fi
done
pathology_n="$(grep -c '"git_outcome": "pathology_pass"' "$ARKS_G17_RECEIPT_OUT" || true)"
resolved_n="$(grep -c '"arks_outcome": "resolved_pass"' "$ARKS_G17_RECEIPT_OUT" || true)"
if [[ "$pathology_n" -lt 145 || "$resolved_n" -lt 145 ]]; then
  echo "G17: expected >=145 pathology_pass and >=145 resolved_pass; got $pathology_n / $resolved_n" >&2
  exit 1
fi
if grep -E '"git_outcome": "fail"|"arks_outcome": "fail"' "$ARKS_G17_RECEIPT_OUT" >/dev/null; then
  echo "G17: receipt contains fail outcomes" >&2
  exit 1
fi
echo "G17: Dz-4 oracle gate PASS (includes Cm/Cx + Dz-1..Dz-3 regression; Deferred catalog IDs claimed)"

echo "G17: PASS"
