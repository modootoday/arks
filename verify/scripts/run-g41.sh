#!/usr/bin/env bash
# G41 — Apache mod_dav dumb HTTP interop (O4).
# Requires Docker; sequential; halt on OOM.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
echo "G41: Apache DAV dumb HTTP suite"
exec docker compose -f verify/docker-compose.yml run --rm g41
