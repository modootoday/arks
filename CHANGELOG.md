# Changelog

All notable changes to `arks` are documented in this file.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and releases adhere to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.2] - 2026-11-28

Completes Wave 1 (`E1`–`E4`) of the `v0.2.0` scope expansion and activates `RubyGems` (`arks`)
zero-token OIDC Trusted Publishing alongside `crates.io`, `PyPI`, `npm`, and GitHub Releases.

### Added

- **Worktree-free history replay (`arks replay`, E3):** pure-Rust in-memory commit replay supporting
  `--onto`, `--advance`, `--contained`, `--ref-action=update|print`, and fail-closed conflict exit
  `1` without mutating the working tree or `.git/index`.
- **Repository maintenance lifecycle (`arks maintenance`, E4):** `run`, `register`, `unregister`,
  `start`, and `stop` honoring `maintenance.lock`, `MAINT-005`/`PACK-005`/`MAINT-006` concurrency
  guards, and race-safe `incremental-repack`.
- **Native subtree management (`arks subtree`, E4):** pure-Rust `add`, `merge`, `split`, `pull`,
  and `push` with standard `git-subtree-dir`/`git-subtree-mainline`/`git-subtree-split` trailers
  and topological DAG projection.

### Fixed

- **RubyGems OIDC release workflow:** pinned `rubygems/configure-rubygems-credentials@v2.2.0` in
  `.github/workflows/release.yml` so `arks` platform gems publish cleanly via zero-token OIDC.

## [0.1.1] - 2026-11-28

Multi-registry OIDC Trusted Publishing release across GitHub Releases, GitHub Pages, `crates.io`
(18 workspace crates), `PyPI` (`arks-cli`), and `npm` (`@modootoday/arks` + 3 scoped platform
binary packages), plus Wave 1 expansion features E1 and E2.

### Added

- **Multi-registry OIDC distribution (D1–D6):** automated GitHub Pages docs
  (`https://modootoday.github.io/arks/`), `crates.io` 18-crate workspace publishing, `PyPI`
  `arks-cli` 3-OS PEP 427/600 wheels, scoped `npm` `@modootoday/arks` + `@modootoday/arks-{linux-x64,darwin-arm64,win32-x64}`
  with provenance, and `RubyGems` `arks` 3-OS platform gems.
- **OAuth 2.0 Device Flow (`arks auth login --web`, E1):** pure-Rust RFC 8628 Device Authorization
  Grant with HTTPS enforcement, `slow_down` backoff, and zero browser/subprocess spawning.
- **GitHub Actions CLI (`workflow`, `run`, `cache`, `variable`, `secret`, E2):** REST API v3
  leaves with pure-Rust NaCl/libsodium `crypto_box_seal` client-side secret encryption and
  zip-slip-safe artifact extraction.

## [0.1.0] - 2026-11-28

First tagged public binary release of `arks` (Option A: unsigned 3-OS GitHub Release archives +
`SHA256SUMS`; `publish = false` retained on workspace crates until crates.io OIDC publishing is
separately enabled by the owner).

### Added

- **Single native Rust binary (`arks`):** zero runtime dependency on stock `git` or `gh`, keeping
  the standard on-disk `.git` repository layout (CELIR not adopted / withdrawn).
- **Git CLI symmetry (pin `git 2.39.5`):** 1,852 / 1,852 options accepted across 185 commands and
  4,446 / 4,549 flag-corpus lines matching stock Git on stdout, exit code, and repository state
  with 0 unreasoned DIFFs (`.spec/evidence/20261128052500-final-pre-release-receipt/`).
- **Three CLI personalities (`git` | `gh` | `arks`):** drop-in alias/shim support (`alias git=arks`,
  `alias gh=arks`) plus unified `arks` surface.
- **Multi-session concurrency:** default product leases (`arks-lease`), high-concurrency
  worktree/ref-shard shape, and shared-index N-way contention support.
- **Git daemon symmetry (`arks daemon`):** 37 / 37 `Parity` options + 2 arks-only options
  (`Missing=0`), byte-identical `upload-archive`, syslog facility/priority, and Windows path/inetd
  support.
- **Independent forge client (`arks pr` / `issue` / `auth` / `milestone` / `ruleset` / …):**
  171 / 171 v1 rows verified in Docker clean-room (`Missing=0`, `Deferred=0`), including 6- and
  12-agent concurrent local forge journeys; GitHub-platform-only groups fail closed with exit 2.
- **Self-hosted server (`arks serve`):** smart/dumb HTTP wire, in-process OpenSSH-compatible SSH
  (ChaCha20-Poly1305), in-process rustls TLS, zero-downtime config rebind (SIGHUP on Unix,
  file-mtime reload on Windows), forge API, and server-side push ruleset enforcement.
- **Remote transports & credentials:** smart/dumb HTTP(S), SSH, `git://`, `--depth` shallow,
  `--filter=blob:none`/`tree:0`, proxy + custom CA trust anchors, `GIT_CONFIG_GLOBAL` precedence,
  per-ref `report-status` push tables, `receive-pack` ref discovery, OS credential stores, and
  OpenPGP / SSH / X.509 commit & tag signing.
- **Optional agent extension (`arks agent`):** LLM review/CEV workflows over plain HTTP and HTTPS
  through the shared `arks-net` TLS/proxy stack.
- **`agy` automated & parallel development governance:** task-scoped `agy/<lane>/<slug>` branch
  taxonomy, Two-Tier parallelism, Step 0 sweep, portable pre-push gate
  (`verify/scripts/agy-governance-check.py`), and advisory `agy-hygiene` CI workflow.

### Fixed

- `format-patch`: RFC 2047 Q-encoding for non-ASCII headers, 8-bit MIME declaration, zero-padded
  patch numbering past nine patches, `git shortlog`-style cover-letter wrapping, `--from`
  committer sender handling, and `In-Reply-To`/`References` threading.
- `push` / `arks-remote`: per-ref `report-status` tables matching `git push` (`! [remote rejected]`,
  `* [new branch]`, shortened source refs) and `GIT_CONFIG_GLOBAL` replacing user global config
  files per `git-config(1)`.
- `verify/scripts/run-symmetry-latest-inventory.sh`: restrict Git command inventory to built-in
  `git --list-cmds=main` with isolated config so third-party `git-*` binaries on `PATH` and user
  aliases do not cause false release-gate failures.
- `arks-hub` / `release.yml`: classify `gh 2.105.0` GitHub Event Streams preview (`event*`) as
  out-of-scope with explicit exit-2 refusal and enforce `block_hub_missing: true` on release tags.
