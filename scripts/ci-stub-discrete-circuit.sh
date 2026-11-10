#!/usr/bin/env bash
# Create parse-time stubs for optional celir-* path-deps in crates/arks-store.
# Cargo resolves optional path deps even when features are OFF; CI has no
# sibling discrete-circuit tree (MUST NOT vendor private CELIR packages).
# Product celir-index feature stays OFF in default GHA jobs.
#
# Layout matches local sibling mount used by verify/docker-compose.yml:
#   <parent-of-repo>/discrete-circuit/{packages,crates}/…
# From crates/arks-store, path ../../../discrete-circuit → that sibling.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DC="$(cd "$REPO_ROOT/.." && pwd)/discrete-circuit"

stub_crate() {
  local dir="$1"
  local name="$2"
  mkdir -p "$dir/src"
  cat >"$dir/Cargo.toml" <<EOF
[package]
name = "${name}"
version = "0.0.0"
edition = "2021"
publish = false
description = "CI parse-time stub — not the real discrete-circuit package"
EOF
  printf '%s\n' '// CI parse-time stub; feature celir-index stays OFF in default GHA.' >"$dir/src/lib.rs"
}

if [[ -f "$DC/packages/celir-data/Cargo.toml" \
   && -f "$DC/packages/celir-index/Cargo.toml" \
   && -f "$DC/crates/celir-index-runtime/Cargo.toml" ]]; then
  echo "discrete-circuit path-deps already present at $DC"
  exit 0
fi

echo "Stubbing discrete-circuit parse-time path-deps at $DC"
stub_crate "$DC/packages/celir-data" "celir-data"
stub_crate "$DC/packages/celir-index" "celir-index"
stub_crate "$DC/crates/celir-index-runtime" "celir-index-runtime"
echo "Stubs ready (feature OFF only; do not enable celir-index against stubs)."