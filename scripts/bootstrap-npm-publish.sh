#!/usr/bin/env bash
# One-time manual npm bootstrap publisher for the 4 arks npm packages.
#
# Why this exists (npm Trusted Publishers):
#   npmjs.com requires each new package (`arks-linux-x64`, `arks-darwin-arm64`,
#   `arks-win32-x64`, and root `arks`) to exist before Trusted Publishing can be
#   bound in Package Settings -> Trusted publishing.
#
# Usage:
#   # 1. Dry-run from downloaded GitHub Release archives:
#   bash scripts/bootstrap-npm-publish.sh --artifacts-dir /path/to/release-assets --dry-run
#
#   # 2. One-time owner bootstrap (never commit or store NPM_TOKEN in GitHub Secrets):
#   npm login
#   gh release download v0.1.0 -D /tmp/arks-v0.1.0 -p 'arks-*'
#   bash scripts/bootstrap-npm-publish.sh --artifacts-dir /tmp/arks-v0.1.0
#   # Then bind Trusted Publisher on npmjs.com for all 4 packages:
#   #   owner=modootoday, repo=arks, workflow=release.yml, environment=npm (allow npm publish).
#
# Cite: .spec/sot/external-distribution-and-registry-publishing.sot.md

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

ARTIFACTS_DIR=""
DRY_RUN=0
PROVENANCE=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --artifacts-dir) ARTIFACTS_DIR="$2"; shift 2 ;;
    --dry-run) DRY_RUN=1; shift ;;
    --provenance) PROVENANCE=1; shift ;;
    -h|--help) sed -n '2,21p' "$0"; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done

if [[ -z "$ARTIFACTS_DIR" ]]; then
  echo "FAIL: --artifacts-dir is required" >&2
  exit 2
fi

VERSION="$(awk -F'"' '/^version *= *"/ { print $2; exit }' Cargo.toml)"
OUT_DIR="$(mktemp -d)"
trap 'rm -rf "$OUT_DIR"' EXIT

node packaging/npm/build_packages.mjs \
  --version "$VERSION" \
  --artifacts-dir "$ARTIFACTS_DIR" \
  --out-dir "$OUT_DIR"

PACKAGES=(arks-linux-x64 arks-darwin-arm64 arks-win32-x64 arks)

already_published() {
  local pkg="$1"
  local ver="$2"
  local code
  code="$(curl -sS -o /dev/null -w '%{http_code}' \
    "https://registry.npmjs.org/${pkg}/${ver}" || echo 000)"
  [[ "$code" == "200" ]]
}

for pkg in "${PACKAGES[@]}"; do
  pkg_dir="$OUT_DIR/$pkg"
  if [[ ! -d "$pkg_dir" ]]; then
    echo "FAIL: missing built package dir $pkg_dir" >&2
    exit 1
  fi
  if [[ "$DRY_RUN" -eq 1 ]]; then
    echo "[dry-run] npm pack $pkg@$VERSION"
    (cd "$pkg_dir" && npm pack --quiet)
    continue
  fi
  if already_published "$pkg" "$VERSION"; then
    echo "[skip] $pkg@$VERSION is already on npm"
    continue
  fi
  echo "[publish] $pkg@$VERSION"
  if [[ "$PROVENANCE" -eq 1 ]]; then
    (cd "$pkg_dir" && npm publish --access public --provenance)
  else
    (cd "$pkg_dir" && npm publish --access public)
  fi
done

echo "=== npm publish step complete (version=$VERSION, dry_run=$DRY_RUN) ==="
