#!/usr/bin/env bash
# One-time manual crates.io bootstrap publisher for the 18 arks workspace crates.
#
# Why this exists (crates.io Trusted Publishing / RFC 3691):
#   crates.io requires each brand-new crate to be published manually once with a
#   temporary API token before Trusted Publishing (OIDC) can be enabled on
#   https://crates.io/crates/<name>/settings.
#
# Usage:
#   # 1. Dry-run (verifies packaging + order without contacting crates.io):
#   bash scripts/bootstrap-crates-publish.sh --dry-run
#
#   # 2. One-time owner bootstrap (never commit or store the token in GitHub Secrets):
#   cargo login   # paste short-lived crates.io publish-new + publish-update token
#   bash scripts/bootstrap-crates-publish.sh
#   # Then revoke the temporary token on crates.io and add Trusted Publisher on each
#   # crate: GitHub owner=modootoday, repo=arks, workflow=release.yml, env=crates.
#
# Cite: .spec/sot/external-distribution-and-registry-publishing.sot.md

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

DRY_RUN=0
WAIT_SECS="${ARKS_CRATES_WAIT_SECS:-20}"

for arg in "$@"; do
  case "$arg" in
    --dry-run) DRY_RUN=1 ;;
    -h|--help)
      sed -n '2,20p' "$0"
      exit 0
      ;;
    *)
      echo "unknown argument: $arg" >&2
      exit 2
      ;;
  esac
done

VERSION="$(awk -F'"' '/^version *= *"/ { print $2; exit }' Cargo.toml)"
if [[ -z "$VERSION" || "$VERSION" == "0.0.0" ]]; then
  echo "FAIL: workspace version in Cargo.toml is '$VERSION' (refusing 0.0.0)" >&2
  exit 1
fi

# Exact topological dependency order (17 library crates + final `arks` CLI binary).
CRATES=(
  arks-config
  arks-hash
  arks-hostfs
  arks-oid
  arks-protocol
  arks-index
  arks-net
  arks-object
  arks-refs
  arks-agent
  arks-pack
  arks-store
  arks-lease
  arks-repo
  arks-hub
  arks-server
  arks-remote
  arks
)

echo "=== arks crates.io topological publish (version=$VERSION, dry_run=$DRY_RUN) ==="

if [[ "$DRY_RUN" -eq 1 ]]; then
  echo "Verifying workspace packaging (--locked) for all ${#CRATES[@]} crates..."
  cargo package --workspace --locked --allow-dirty
  for c in "${CRATES[@]}"; do
    echo "  [dry-run OK] $c $VERSION"
  done
  echo "=== DRY-RUN OK (zero crates uploaded) ==="
  exit 0
fi

already_published() {
  local crate="$1"
  local ver="$2"
  local code
  code="$(curl -sS -o /dev/null -w '%{http_code}' -A 'arks-release-publisher/0.1' \
    "https://crates.io/api/v1/crates/${crate}/${ver}" || echo 000)"
  [[ "$code" == "200" ]]
}

for crate in "${CRATES[@]}"; do
  if already_published "$crate" "$VERSION"; then
    echo "[skip] $crate $VERSION is already on crates.io"
    continue
  fi
  echo "[publish] $crate $VERSION"
  cargo publish -p "$crate" --locked
  echo "  waiting ${WAIT_SECS}s for crates.io index propagation..."
  sleep "$WAIT_SECS"
done

echo "=== All ${#CRATES[@]} crates published for version $VERSION ==="
echo "Next: revoke your temporary crates.io API token and bind Trusted Publishing on:"
for crate in "${CRATES[@]}"; do
  echo "  https://crates.io/crates/${crate}/settings"
done
echo "  (owner=modootoday, repo=arks, workflow=release.yml, environment=crates)"
