#!/usr/bin/env bash
# G15 — git-server / production-server (M11) + G14/G7/G5 regression + gix-exit.
# Does not redefine M9. Lease not required.
set -euo pipefail

cd /workspace

if ! command -v git >/dev/null; then
  echo "G15: stock git missing in image" >&2
  exit 1
fi
echo "G15: stock git $(git --version)"
echo "G15: rustc $(rustc --version)"
echo "G15: CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-2}"

export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"

echo "G15: gix-exit scan"
if cargo metadata --format-version 1 --no-deps 2>/dev/null | grep -E '"name":\s*"(gix|gitoxide)' >/dev/null; then
  echo "G15: forbidden gix/gitoxide package name in workspace members" >&2
  exit 1
fi
meta="$(cargo metadata --format-version 1)"
if echo "$meta" | grep -E '"name":\s*"gix|"name":\s*"gitoxide|"name":\s*"gix-' >/dev/null; then
  echo "G15: forbidden gix/gitoxide edge in cargo metadata" >&2
  exit 1
fi
# arks-server must not depend on arks-lease / gix / celir (optional path-deps on
# arks-store may appear in workspace metadata when discrete-circuit is mounted;
# that is parse-time only — refuse only if arks-server itself edges them).
if echo "$meta" | python3 -c "
import json,sys
m=json.load(sys.stdin)
forbid={'arks-lease','gix','gitoxide','celir'}
for p in m['packages']:
  if p['name']!='arks-server':
    continue
  for d in p.get('dependencies',[]):
    n=d.get('name') or ''
    if n in forbid or n.startswith('gix-') or n.startswith('celir-'):
      sys.exit(1)
  sys.exit(0)
sys.exit(1)
"; then
  echo "G15: arks-server deps OK (no gix/celir/lease)"
else
  echo "G15: arks-server has forbidden dep" >&2
  exit 1
fi
# Default feature tree must not pull celir (feature celir-index stays OFF).
if cargo tree -p arks-store --edges normal 2>/dev/null | grep -Ei 'celir' >/dev/null; then
  echo "G15: FAIL — default arks-store tree has celir edge" >&2
  exit 1
fi
echo "G15: gix-exit / no-celir / no-lease-dep OK"

echo "G15: build arks binary"
cargo build -p arks

echo "G15: arks-server unit tests"
cargo test -p arks-server "$@"

echo "G15: git-server suite vs stock git (verify/g15)"
cargo test --manifest-path verify/g15/Cargo.toml "$@"

echo "G15: G14 CLI-parity regression"
cargo test --manifest-path verify/g14/Cargo.toml "$@"

echo "G15: G7 remote-client regression"
cargo test --manifest-path verify/g7/Cargo.toml "$@"

echo "G15: G5 protocol regression"
cargo test --manifest-path verify/g5/Cargo.toml "$@"
cargo test -p arks-protocol "$@"

echo "G15: PASS"
