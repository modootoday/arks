# Changelog

All notable changes to `arks` are documented in this file.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and releases adhere to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
