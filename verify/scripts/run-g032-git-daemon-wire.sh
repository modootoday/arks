#!/usr/bin/env bash
# G32 — git:// daemon (M25 / FW-SRV-GIT)
set -euo pipefail
cd /workspace
if ! command -v git >/dev/null; then
  echo "G32: stock git missing" >&2
  exit 1
fi
echo "G32: stock git $(git --version)"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
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
  echo "G32: arks-server deps OK"
else
  echo "G32: arks-server has forbidden dep" >&2
  exit 1
fi
echo "G32: build arks"
cargo build -p arks
echo "G32: git-daemon suite"
cargo test --manifest-path verify/gates/g032-git-daemon-wire/Cargo.toml "$@"
echo "G32: PASS"
