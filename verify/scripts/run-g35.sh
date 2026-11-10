#!/usr/bin/env bash
# G35 — client shallow --depth (FW-G9 / G9 Claimed subset)
set -euo pipefail
cd /workspace
if ! command -v git >/dev/null; then
  echo "G35: stock git missing" >&2
  exit 1
fi
echo "G35: stock git $(git --version)"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
meta="$(cargo metadata --format-version 1)"
if echo "$meta" | python3 -c "
import json,sys
m=json.load(sys.stdin)
for p in m['packages']:
  if p['name']=='arks-remote':
    for d in p.get('dependencies',[]):
      n=d.get('name','')
      if n=='gix' or n.startswith('gix-') or n=='celir' or n.startswith('celir-'):
        print(n); sys.exit(1)
sys.exit(0)
"; then
  echo "G35: arks-remote deps OK"
else
  echo "G35: arks-remote has forbidden dep" >&2
  exit 1
fi
echo "G35: build arks"
cargo build -p arks -p arks-remote
echo "G35: shallow depth suite"
cargo test --manifest-path verify/g35/Cargo.toml "$@"
echo "G35: PASS"
