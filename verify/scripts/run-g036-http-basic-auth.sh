#!/usr/bin/env bash
# G36 — optional HTTP Basic on arks smart HTTP (FW-AUTH-PROXY)
set -euo pipefail
cd /workspace
if ! command -v git >/dev/null; then
  echo "G36: stock git missing" >&2
  exit 1
fi
echo "G36: stock git $(git --version)"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
echo "G36: gix / arks-server dep honesty"
meta="$(cargo metadata --format-version 1)"
if echo "$meta" | python3 -c "
import json,sys
m=json.load(sys.stdin)
for p in m['packages']:
  if p['name']=='arks-server':
    for d in p.get('dependencies',[]):
      n=d.get('name','')
      if n=='arks-lease' or n=='gix' or n.startswith('gix-') or n=='celir' or n.startswith('celir-'):
        print(n); sys.exit(1)
sys.exit(0)
"; then
  echo "G36: arks-server deps OK (no gix/celir/lease)"
else
  echo "G36: arks-server has forbidden dep" >&2
  exit 1
fi
echo "G36: build arks"
cargo build -p arks
echo "G36: arks-server unit"
cargo test -p arks-server "$@"
echo "G36: HTTP Basic suite"
cargo test --manifest-path verify/gates/g36/Cargo.toml "$@"
echo "G36: PASS"
