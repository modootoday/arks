#!/usr/bin/env bash
# Thin PATH helper for gh personality — NOT a second published binary.
# Cite: `.spec/sot/arks-cli-personality-factory.sot.md`
#
# Preferred: hard-link/copy the `arks` binary as `gh` on PATH (argv0 → gh).
# This script is the fallback when a hard link is inconvenient:
#   ln -s "$(pwd)/scripts/shim-gh.sh" "$HOME/bin/gh"
#
# Forces ARKS_PERSONALITY=gh (env wins over argv0) then execs `arks`.
set -euo pipefail
export ARKS_PERSONALITY=gh
if command -v arks >/dev/null 2>&1; then
  exec arks "$@"
fi
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
if [[ -x "$ROOT/target/debug/arks" ]]; then
  exec "$ROOT/target/debug/arks" "$@"
fi
if [[ -x "$ROOT/target/release/arks" ]]; then
  exec "$ROOT/target/release/arks" "$@"
fi
echo "shim-gh: arks binary not found on PATH or in target/{debug,release}" >&2
exit 2
