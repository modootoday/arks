---
id: sot:root/external-distribution-and-registry-publishing
title: External distribution and multi-registry OIDC publishing (D1–D6)
status: active
created: 2026-11-28T07:00:00Z
updated: 2026-11-28T07:45:00Z
tags: [release, distribution, oidc, trusted-publishing, github-pages, crates-io, pypi, npm, rubygems]
verify: manual
rel:
  depends_on:
    - sot:root/product-naming
    - sot:root/product-intent
    - sot:root/public-surface
    - sot:root/release-symmetry-precheck
    - sot:root/product-finalization-and-release-readiness
    - sot:root/autonomous-operation
    - sot:root/agy-automated-development
    - research:root/20261110085731
  governs:
    - .spec/plan/20261128070000-external-distribution-d1-d6.plan.md
    - .github/workflows/pages.yml
    - .github/workflows/release.yml
    - packaging/
    - scripts/release-channel-readiness.sh
---

# External distribution and multi-registry OIDC publishing (D1–D6)

Version **0.1.2**. Normative specification for shipping **`arks`** across every KEEP external
distribution surface beyond GitHub Releases (`v0.1.0` live): **GitHub Pages**, **`crates.io`**,
**`PyPI`**, **`npm`**, and **`RubyGems`** — using **GitHub Actions OIDC Trusted Publishing**
(zero long-lived registry tokens in repo secrets) while preserving the single-binary product
invariant ([[sot:root/product-naming]]).

Design research baseline: [[research:root/20261110085731]].
Executable queue: [[plan:root/20261128070000]].

## 1. Channel scope and verified package names (2026-11-28)

Live registry checks on 2026-11-28 confirmed package naming across every KEEP channel:
- `pypi.org/simple/arks/` returned HTTP 200 due to an existing zero-release project, so PyPI uses
  `arks-cli` while installing the `arks` binary (respecting [[sot:root/product-naming]]'s
  no-`git`-substring rule).
- `registry.npmjs.org/arks` triggered npm's typosquatting guard against `arg`/`args`/`yargs`, so all
  4 npm packages live cleanly under the official `@modootoday/*` scope (`@modootoday/arks` plus
  `@modootoday/arks-{linux-x64,darwin-arm64,win32-x64}`), installing the `arks` binary.

| Channel | Distribution unit(s) | Scope | Auth model | First-publish bootstrap rule (primary source) |
|---------|----------------------|-------|------------|-----------------------------------------------|
| **GitHub Releases** | `arks-<target>.tar.gz` / `.zip` + `SHA256SUMS` (3 OS, unsigned) | **LIVE (`v0.1.0`)** | `GITHUB_TOKEN` (`contents: write`) | Automated on `v*` tag push |
| **GitHub Pages** | `https://modootoday.github.io/arks/` from `docs/` (`jekyll-theme-modootoday`) | **KEEP (D1)** | `GITHUB_TOKEN` (`pages: write`, `id-token: write`) | Automated via `.github/workflows/pages.yml` (`build_type: workflow`) |
| **crates.io** | `arks` (single `[[bin]]`) + 17 library-only `arks-*` workspace crates | **KEEP (D2)** | OIDC `rust-lang/crates-io-auth-action@v1` (`environment: crates`) | **Manual first publish once** with temporary API token, then configure Trusted Publisher per crate and revoke token ([crates.io Trusted Publishing](https://crates.io/docs/trusted-publishing), [RFC 3691](https://rust-lang.github.io/rfcs/3691-trusted-publishing-cratesio.html)) |
| **PyPI** | `arks-cli` platform wheels (`manylinux_2_39_x86_64`, `macosx_11_0_arm64`, `win_amd64`; installs `arks` binary) | **KEEP (D3)** | OIDC `pypa/gh-action-pypi-publish@release/v1` (`environment: pypi`) | **Zero-token Pending Trusted Publisher** on `pypi.org` before first tag ([PyPI Trusted Publishers](https://docs.pypi.org/trusted-publishers/adding-a-publisher/)) |
| **npm** | `@modootoday/arks` + 3 platform packages (`@modootoday/arks-linux-x64`, `@modootoday/arks-darwin-arm64`, `@modootoday/arks-win32-x64`) | **KEEP (D4)** | OIDC `npm publish --access public --provenance` (Node 24, `npm >= 11.5.1`, `environment: npm`) | **Manual first publish once** (`npm login` + `npm publish --access public`), then bind Trusted Publisher on `npmjs.com` for each of the 4 packages and allow `npm publish` ([npm Trusted Publishers](https://docs.npmjs.com/trusted-publishers/)) |
| **RubyGems** | `arks` platform gems (`x86_64-linux`, `arm64-darwin`, `x64-mingw-ucrt`) | **KEEP (D5)** | OIDC `rubygems/configure-rubygems-credentials@v1` + `gem push` (`environment: rubygems`) | **Zero-token Pending Trusted Publisher** on `rubygems.org` before first tag ([RubyGems Trusted Publishing](https://guides.rubygems.org/trusted-publishing/)) |
| **OS signing channels** (`winget`, `apt` GPG, `Chocolatey`, Authenticode, Apple notarization, Homebrew tap) | — | **OUT / Deferred** | — | Excluded per [[research:root/20261110085731]] § Current release-channel scope |

## 2. Invariants (normative)

1. **Single published binary:** Only `crates/arks` declares `[[bin]]` (`arks`). The 17 internal
   workspace crates (`arks-config` … `arks-remote`) are **library-only** crates published on
   `crates.io` solely so `cargo install arks` resolves cleanly. No separate `arks-daemon`,
   `arks-hub`, or `arks-agent` binary is ever packaged or published.
2. **Bit-identical wrapper binaries:** PyPI wheels, npm platform packages, and RubyGems platform
   gems MUST pack the **exact prebuilt `arks` binary bytes** produced by `.github/workflows/release.yml`
   job `dist` — never download binaries over the network during `postinstall` / install hooks, and
   never spawn stock `git` or `gh`.
3. **Zero long-lived registry tokens in repository secrets:** `.github/workflows/release.yml` MUST
   use OIDC (`permissions.id-token: write`) only. Where a registry requires an initial manual
   first-publish (`crates.io`, `npm`), the owner runs a local bootstrap script (`scripts/bootstrap-*.sh`)
   interactively once, binds the Trusted Publisher in the registry UI, and revokes the temporary
   token — never storing tokens in GitHub Secrets.
4. **Hardened GitHub Environments:** All 4 registry Environments (`crates`, `npm`, `pypi`,
   `rubygems`) on `modootoday/arks` MUST enforce:
   - Custom deployment branch/tag policy restricted to `v*` tags only (preventing arbitrary branches
     from minting registry OIDC tokens).
   - Required reviewer (`myso-kr`) so live registry publishes require deliberate owner approval.
5. **Real packaging verification in dry-run:** `.github/workflows/release.yml` (`workflow_dispatch`,
   `dry_run=true`) MUST build and locally validate all 4 channel artifacts from the 3-OS `dist`
   binaries (npm tarballs + Linux `npx`/bin smoke, PyPI wheels + isolated `pip install --no-index`
   smoke, RubyGems `.gem` builds, and `cargo package --list`) without creating a GitHub Release or
   calling any external registry publish endpoint.
6. **Honesty boundary:** Local packaging smoke tests and GHA `dry_run=true` greens prove **packaging
   and ops readiness** only — NEVER claim live `crates.io` / `PyPI` / `npm` / `RubyGems` publish
   PASS until the corresponding `v*` tag OIDC job succeeds against the live registry.

## 3. Packaging architecture

### 3.1 `crates.io` (D2)

- Root `Cargo.toml` `[workspace.dependencies]` pins explicit `version = "<workspace-version>"`
  alongside every internal `path = "crates/..."` dependency.
- All 18 workspace crates inherit `version.workspace = true`, `license.workspace = true`,
  `repository.workspace = true`, and carry valid `description`, `readme`, `homepage`, `keywords`,
  and `categories` metadata (`publish` enabled).
- Topological publish order (18 crates):
  1. `arks-config`, `arks-hash`, `arks-hostfs`, `arks-oid`, `arks-protocol`
  2. `arks-index`, `arks-net`, `arks-object`, `arks-refs`
  3. `arks-agent`, `arks-pack`
  4. `arks-store`
  5. `arks-lease`, `arks-repo`
  6. `arks-hub`, `arks-server`
  7. `arks-remote`
  8. `arks`
- `publish-crates` in `.github/workflows/release.yml` publishes all 18 crates in topological order
  using `rust-lang/crates-io-auth-action@v1` (short-lived OIDC token), skipping crates whose target
  version is already present on `crates.io` so retries are idempotent.

### 3.2 PyPI binary wheels (D3)

- Tree: `packaging/pypi/` (`pyproject.toml`, `build_wheels.py`).
- PyPI project name: `arks-cli` (normalized PEP 427 wheel prefix `arks_cli`; installs the `arks`
  binary onto `PATH`).
- `build_wheels.py` uses Python 3 standard library (`zipfile`, `hashlib`, `base64`) to pack the
  exact `dist` archives into PEP 427 / PEP 600 binary wheels:
  - `arks_cli-<ver>-py3-none-manylinux_2_39_x86_64.whl` (`arks_cli-<ver>.data/scripts/arks`, mode `0755`)
  - `arks_cli-<ver>-py3-none-macosx_11_0_arm64.whl` (`arks_cli-<ver>.data/scripts/arks`, mode `0755`)
  - `arks_cli-<ver>-py3-none-win_amd64.whl` (`arks_cli-<ver>.data/scripts/arks.exe`, mode `0755`)
- Placing the native binary in `<dist>.data/scripts/` exposes `arks` directly on `PATH` inside
  virtualenvs, `pipx`, and `uvx` with zero Python wrapper startup overhead.

### 3.3 npm platform packages + root shim (D4)

- Tree: `packaging/npm/` (`build_packages.mjs`, `bin/arks.js`).
- `build_packages.mjs` generates 4 publishable scoped directories under `@modootoday/*` from the
  3-OS `dist` archives:
  - `@modootoday/arks-linux-x64` (`"os": ["linux"]`, `"cpu": ["x64"]`, `bin/arks`)
  - `@modootoday/arks-darwin-arm64` (`"os": ["darwin"]`, `"cpu": ["arm64"]`, `bin/arks`)
  - `@modootoday/arks-win32-x64` (`"os": ["win32"]`, `"cpu": ["x64"]`, `bin/arks.exe`)
  - `@modootoday/arks` (root package: `"bin": { "arks": "bin/arks.js" }`, `"optionalDependencies"`
    pinned to the exact version of the 3 `@modootoday/arks-*` platform packages; `bin/arks.js`
    resolves the matching platform package via `require.resolve` and executes the native binary
    directly with inherited stdio and exit code — zero network download at install time).

### 3.4 RubyGems multi-platform gems (D5)

- Tree: `packaging/rubygems/` (`build_gems.rb`, `exe/arks`).
- `build_gems.rb` packs 3 platform `.gem` files from the 3-OS `dist` archives:
  - `arks-<ver>-x86_64-linux.gem` (`libexec/arks` + thin `exe/arks` `Kernel.exec` shim)
  - `arks-<ver>-arm64-darwin.gem` (`libexec/arks` + thin `exe/arks` `Kernel.exec` shim)
  - `arks-<ver>-x64-mingw-ucrt.gem` (`libexec/arks.exe` + thin `exe/arks` shim)
- `publish-rubygems` in `.github/workflows/release.yml` uses
  `rubygems/configure-rubygems-credentials@v1` (OIDC, `environment: rubygems`) and runs `gem push`
  on all 3 built `.gem` files.

### 3.5 GitHub Pages (D1)

- Workflow `.github/workflows/pages.yml` builds `docs/` with `actions/jekyll-build-pages@v1` and
  deploys via `actions/deploy-pages@v4` under `environment: github-pages`.
- Live URL: `https://modootoday.github.io/arks/`.

## 4. Verification

1. **D1:** `https://modootoday.github.io/arks/` returns HTTP 200; all 4 GitHub Environments
   (`crates`, `npm`, `pypi`, `rubygems`) report custom `v*` tag policy and required reviewer
   `myso-kr` via `gh api`.
2. **D2:** `cargo package --locked` / workspace tests / clippy pass across all 18 crates;
   `scripts/bootstrap-crates-publish.sh --dry-run` exits 0.
3. **D3:** `packaging/pypi/build_wheels.py` produces valid PEP 427 wheels; isolated `venv`
   `pip install --no-index` + `arks --version` exits 0 with matching semver.
4. **D4:** `packaging/npm/build_packages.mjs` + `npm pack` produces 4 valid tarballs; isolated
   `npm install --offline` + `arks --version` exits 0 with matching semver.
5. **D5:** `packaging/rubygems/build_gems.rb` produces 3 valid `.gem` packages (`gem spec` valid).
6. **D6:** Live GHA `release.yml` (`workflow_dispatch`, `dry_run=true`) passes all 4 packaging
   dry-run checks; scrubbed receipt committed under `.spec/evidence/`; owner receives a `needs-owner`
   issue containing exact one-time Trusted Publisher registration / bootstrap instructions for the
   first live multi-registry tag (`v0.1.1`).
