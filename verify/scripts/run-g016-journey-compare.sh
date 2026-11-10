#!/usr/bin/env bash
# G16 — journey-compare (func + timing). Additive; not a production unlock.
# SoT: .spec/sot/clean-room-journey-compare.sot.md
set -euo pipefail

cd /workspace

if ! command -v git >/dev/null; then
  echo "G16: stock git missing in image" >&2
  exit 1
fi
echo "G16: stock git $(git --version)"
echo "G16: rustc $(rustc --version)"
# Prefer compose default (CARGO_BUILD_JOBS=1) — dual agents with jobs=2 OOM → 137.
echo "G16: CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-1}"

export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"

echo "G16: gix-exit scan"
if cargo metadata --format-version 1 --no-deps 2>/dev/null | grep -E '"name":\s*"(gix|gitoxide)' >/dev/null; then
  echo "G16: forbidden gix/gitoxide package name in workspace members" >&2
  exit 1
fi
meta="$(cargo metadata --format-version 1)"
if echo "$meta" | grep -E '"name":\s*"gix|"name":\s*"gitoxide|"name":\s*"gix-' >/dev/null; then
  echo "G16: forbidden gix/gitoxide edge in cargo metadata" >&2
  exit 1
fi
echo "G16: gix-exit OK"

echo "G16: build arks binary (release; compile excluded from timings)"
cargo build -p arks --release

# Warm text pages for both peers (compile already excluded; not timed).
ARKS_BIN="$(find "${CARGO_TARGET_DIR:-target-cleanroom}/release" target-cleanroom/release target/release -name arks -type f 2>/dev/null | head -n1 || true)"
if [[ -n "${ARKS_BIN}" && -x "${ARKS_BIN}" ]]; then
  "${ARKS_BIN}" --help >/dev/null 2>&1 || true
fi
git version >/dev/null 2>&1 || true

mkdir -p verify/gates/g016-journey-compare/out
export ARKS_JOURNEY_METRICS_OUT="${ARKS_JOURNEY_METRICS_OUT:-/workspace/verify/gates/g016-journey-compare/out/metrics.json}"
export ARKS_JOURNEY_VENUE="${ARKS_JOURNEY_VENUE:-clean-room}"
export ARKS_JOURNEY_IMAGE_TAG="${ARKS_JOURNEY_IMAGE_TAG:-arks-verify-m1:0.1.0}"
# Authoritative default: multi-iteration median + p95 (SoT v0.2.0). Override for advisory loops.
export ARKS_JOURNEY_ITERATIONS="${ARKS_JOURNEY_ITERATIONS:-3}"
echo "G16: ARKS_JOURNEY_ITERATIONS=${ARKS_JOURNEY_ITERATIONS}"
# Prefer lock digest when caller does not override (no python/jq required).
if [[ -z "${ARKS_JOURNEY_IMAGE_DIGEST:-}" && -f verify/images.lock.json ]]; then
  ARKS_JOURNEY_IMAGE_DIGEST="$(
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
  # Strip residual CR from CRLF lock files on Windows-hosted mounts.
  ARKS_JOURNEY_IMAGE_DIGEST="${ARKS_JOURNEY_IMAGE_DIGEST//$'\r'/}"
fi
export ARKS_JOURNEY_IMAGE_DIGEST="${ARKS_JOURNEY_IMAGE_DIGEST:-}"
export ARKS_JOURNEY_GIT_VERSION="$(git --version)"
export ARKS_JOURNEY_REVISION="$(git -C /workspace rev-parse HEAD 2>/dev/null || echo unknown)"
export ARKS_JOURNEY_PROFILE=release

echo "G16: journey-compare suite (verify/gates/g016-journey-compare; release harness — lower RSS vs debug)"
# Release harness avoids debuginfo RSS blow-ups that SIGKILL (137) Docker Desktop.
cargo test --release --manifest-path verify/gates/g016-journey-compare/Cargo.toml -- --nocapture "$@"

if [[ -f "$ARKS_JOURNEY_METRICS_OUT" ]]; then
  echo "G16: metrics written to $ARKS_JOURNEY_METRICS_OUT"
  head -n 40 "$ARKS_JOURNEY_METRICS_OUT" || true
else
  echo "G16: metrics file missing: $ARKS_JOURNEY_METRICS_OUT" >&2
  exit 1
fi

echo "G16: PASS"
