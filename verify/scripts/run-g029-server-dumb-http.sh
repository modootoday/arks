#!/usr/bin/env bash
# G29 — server dumb HTTP (M25 / FW-SRV-DUMB)
set -euo pipefail
cd /workspace
if ! command -v git >/dev/null; then
  echo "G29: stock git missing" >&2
  exit 1
fi
echo "G29: stock git $(git --version)"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
echo "G29: gix / arks-server dep honesty"
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
  echo "G29: arks-server deps OK (no gix/celir/lease)"
else
  echo "G29: arks-server has forbidden dep" >&2
  exit 1
fi
echo "G29: build arks"
cargo build -p arks
echo "G29: arks-server unit"
cargo test -p arks-server "$@"
echo "G29: dumb HTTP suite"
cargo test --manifest-path verify/gates/g029-server-dumb-http/Cargo.toml "$@"
echo "G29: PASS"
