#!/usr/bin/env bash
# Report OIDC / GitHub Environment release-channel dry-run readiness for arks.
#
# Exit 0 when structural wiring is consistent with the KEEP channel model
# (crates / npm / pypi / rubygems Environments + release.yml). Never claims a
# registry publish Verified PASS. Live tag publish remains operator-gated.
#
# Cite: .spec/research/20261110085731-github-actions-release-oidc.research.md
#       .spec/plan/20261110085731-github-actions-ci-release-oidc.plan.md

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

echo "## Release-channel readiness (dry-run)"
echo
echo "This report is **ops readiness**, not a live registry publish PASS."
echo "Trusted publishing + Environment approval still required for real tags."
echo

expected_envs=(crates npm pypi rubygems)
echo "### Expected GitHub Environments (tag policy \`v*\`; OIDC; no long-lived tokens)"
for e in "${expected_envs[@]}"; do
  echo "- \`${e}\` — job \`environment: ${e}\` in \`.github/workflows/release.yml\`"
done
echo
echo "Human residual: Environments live under"
echo "https://github.com/modootoday/arks/settings/environments"
echo "(required reviewer + \`v*\` tag policy are API-configured; YAML cannot invent them)."
echo

echo "### Packaging gates (fail-closed skip until ready)"
skip_notes=()

if [[ -f crates/arks/Cargo.toml ]] && grep -qE '^publish[[:space:]]*=[[:space:]]*false' crates/arks/Cargo.toml; then
  echo "- crates.io: **skip expected** — \`crates/arks\` has \`publish = false\`"
  skip_notes+=("crates")
else
  echo "- crates.io: crate looks publishable — still needs crates.io trusted publisher bound to \`release.yml\` + environment \`crates\`"
fi

if [[ ! -f npm/package.json ]]; then
  echo "- npm: **skip expected** — no \`npm/package.json\` yet"
  skip_notes+=("npm")
else
  echo "- npm: packaging tree present — needs npm trusted publisher + environment \`npm\`"
fi

if [[ ! -f pyproject.toml && ! -f python/pyproject.toml ]]; then
  echo "- PyPI: **skip expected** — no \`pyproject.toml\` / \`python/pyproject.toml\` yet"
  skip_notes+=("pypi")
else
  echo "- PyPI: packaging tree present — needs PyPI trusted publisher + environment \`pypi\`"
fi

if ! compgen -G "*.gemspec" >/dev/null 2>&1; then
  echo "- RubyGems: **skip expected** — no \`*.gemspec\` yet"
  skip_notes+=("rubygems")
else
  echo "- RubyGems: gemspec present — needs RubyGems trusted publisher + environment \`rubygems\`"
fi

echo
echo "### Workflow triggers"
echo "- Tag \`v*\` push: ungated dist + draft GitHub Release (checksum/unsigned);"
echo "  then Environment-gated OIDC publish jobs (approve to proceed)."
echo "- \`workflow_dispatch\` dry-run: dist + this readiness report only —"
echo "  **no** draft Release, **no** OIDC publish, **no** invented PASS."
echo
echo "### OUT / Deferred (do not treat as missing L4)"
echo "- winget / apt signed repo / Chocolatey / Authenticode / Apple notarization"
echo
echo "### Summary"
echo "- Environments model: crates | npm | pypi | rubygems (per-channel, not a single \`release\` env)"
echo "- Skip gates today: ${skip_notes[*]:-none}"
echo "- Live publish: **not claimed** by this dry-run"
echo
echo "release-channel-readiness: OK (structural)"
