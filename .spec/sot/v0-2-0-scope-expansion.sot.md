---
id: sot:root/v0-2-0-scope-expansion
title: v0.2.0 scope expansion (E1–E8) — OAuth Device Flow, Actions CLI, replay, maintenance/subtree, reftable, LFS, serve web UI
status: active
created: 2026-11-28T09:25:00Z
updated: 2026-11-28T09:25:00Z
tags: [sot, v0-2-0, scope-expansion, reftable, git-replay, git-lfs, oauth-device-flow, github-actions, maintenance, subtree, self-host-ui]
verify: docker
rel:
  depends_on:
    - sot:root/product-naming
    - sot:root/product-intent
    - sot:root/git-compat-surface
    - sot:root/cli-surface
    - sot:root/arks-hub-product-intent
    - sot:root/arks-self-host-server
    - sot:root/product-verification-clean-room
    - sot:root/production-residual-register
    - research:root/20261128092500
  governs:
    - .spec/plan/20261128092500-v0-2-0-scope-expansion-e1-e8.plan.md
---

# v0.2.0 scope expansion (E1–E8)

Version **0.1.0** (**Design** — not Claimed). Normative specification for expanding `arks` beyond
the `v0.1.x` baseline to remove the top real-world adoption friction points while deepening
multi-agent concurrency and self-host ergonomics.

Technical references and algorithms: [[research:root/20261128092500]].
Executable ladder: [[plan:root/20261128092500]].

> **Honesty notice:** Every epic E1–E8 in this document starts in **Design** status. No surface
> becomes **Claimed** or **Verified** until implementation lands with matching unit tests, 3-OS CI
> greens, and Docker clean-room receipts under `.spec/evidence/` (per
> [[sot:root/product-verification-clean-room]] and rules `28`/`29`).

---

## 1. Scope and Epics (E1–E8)

| Epic | Wave | Surface | Target crate(s) | Current status | Verification gate |
|------|------|---------|-----------------|----------------|-------------------|
| **E1** | Wave 1 | OAuth 2.0 Device Authorization Grant (`arks auth login --web`) | `arks-hub`, `arks` | **Design** | Docker fixture OAuth server E2E (RFC 8628 state machine) |
| **E2** | Wave 1 | GitHub Actions CLI (`run`, `workflow`, `cache`, `secret`, `variable`) + pure-Rust NaCl sealed-box | `arks-hub`, `arks` | **Design** | Docker fixture REST API E2E + RFC/libsodium sealed-box KAT |
| **E3** | Wave 1 | Worktree-free history replay (`arks replay`) | `arks-repo`, `arks` | **Design** | Docker clean-room interop against stock `git replay` (Git 2.44+) |
| **E4** | Wave 1 | `arks maintenance` + `arks subtree` + 5-command Partial corpus promotion (`fast-import`, `index-pack`, `http-fetch`, `http-push`, `help`) | `arks-repo`, `arks-pack`, `arks` | **Design** | Docker clean-room parity against stock Git |
| **E5** | Wave 2 | Git Reftable backend (`.git/reftable/`, `extensions.refStorage = reftable`, `arks refs migrate`/`verify`) | `arks-refs`, `arks-repo`, `arks` | **Design** | Docker clean-room bidirectional interop with stock Git 2.46+ + multi-agent contention benchmark |
| **E6** | Wave 2 | Native Git LFS client (`arks lfs` + in-process `clean`/`smudge`/`filter-process` + Batch/Basic transfer) | `arks-remote`, `arks-repo`, `arks` | **Design** | Docker clean-room interop with stock `git-lfs` + LFS test server |
| **E7** | Wave 2 | Self-host LFS endpoint (`arks serve` `info/lfs` Batch + object store) | `arks-server` | **Design** | Docker clean-room E2E (`arks lfs` and stock `git-lfs` against `arks serve`) |
| **E8** | Wave 3 | Single-binary embedded SSR web UI for `arks serve` (`--no-web-ui` opt-out) | `arks-server` | **Design** | Docker clean-room HTTP/HTML/XSS/CSP E2E |

---

## 2. Normative Requirements

### 2.1 Global invariants (all epics)
1. **Single published binary:** All new capabilities MUST dispatch from the single `arks` binary
   (`crates/arks`). No separate helper binaries (`arks-lfs`, `arks-hub`, `arks-web`) may be shipped.
2. **Zero stock `git`/`gh`/`git-lfs` spawning:** Product codepaths MUST NEVER spawn stock `git`,
   `gh`, or `git-lfs` subprocesses. Stock tools are clean-room test oracles only.
3. **Standard `.git` interchange:** Reftable (`E5`) and LFS (`E6`/`E7`) MUST conform strictly to
   their documented public Git / Git-LFS on-disk and wire formats so stock `git` and `git-lfs` can
   read and write the same repository without corruption.
4. **CELIR boundary:** CELIR remains permanently withdrawn ([[adr:root/20261119200000]]) and MUST
   NOT enter the Cargo graph.
5. **Fail-closed honesty:** Any unimplemented flag, unsupported transfer adapter, unknown Git
   repository extension, or unsupported forge endpoint MUST exit non-zero without mutating repo
   state.

### 2.2 E1 — OAuth 2.0 Device Authorization Grant
- MUST implement RFC 8628 §3.1–§3.5 (`device_code`, `user_code`, `verification_uri`,
  `authorization_pending`, `slow_down` +5s interval increase, `expired_token`, `access_denied`).
- MUST require HTTPS for non-loopback OAuth endpoints; MUST never print `device_code` or raw
  `access_token` to terminal logs.
- MUST store obtained tokens through `arks-hub`'s existing credential/wallet boundary.

### 2.3 E2 — GitHub Actions CLI (`run`, `workflow`, `cache`, `secret`, `variable`)
- MUST supersede the prior `OUT` exit-2 refusal for these 5 groups in `arks-hub` and `gh`
  personality dispatch while keeping truly proprietary groups (`codespace`, `copilot`, `extension`,
  `agent-task`, `event*`, `skill`) explicitly refused with exit 2.
- `secret set` MUST encrypt secrets client-side using NaCl/libsodium `crypto_box_seal` (X25519 +
  BLAKE2b-192 nonce + XSalsa20Poly1305) in pure Rust (no C libsodium FFI) against the fetched
  repository/org/environment public key before HTTP transmission.
- `run download` MUST reject zip-slip traversal paths (`..` or absolute paths) fail-closed.

### 2.4 E3 — Worktree-Free History Replay (`arks replay`)
- MUST NOT touch the working tree or `.git/index` under any flag combination.
- MUST support `--onto`, `--advance`, `--contained`, `--ref-action=update|print`, and conflict
  fail-closed exit `1` without mutating any ref.
- `--ref-action=print` output MUST be byte-compatible with `arks update-ref --stdin` and stock
  `git update-ref --stdin`.

### 2.5 E4 — `arks maintenance`, `arks subtree`, and 5-Command Corpus Promotion
- `arks maintenance` MUST support `register`, `unregister`, `start`, `stop`, and `run` with
  `incremental`, `geometric`, and `gc` strategies, honoring `maintenance.lock` and race-safe
  two-step MIDX `incremental-repack`.
- `arks subtree` MUST support `add`, `pull`, `push`, `merge`, and `split` with standard
  `git-subtree-dir` / `git-subtree-split` trailers and cached incremental `split`.
- Existing commands `fast-import`, `index-pack`, `http-fetch`, `http-push`, and `help` MUST gain
  Docker clean-room parity corpus rows to promote them from `Partial` to `Verified`.

### 2.6 E5 — Git Reftable Backend (`arks-refs`)
- MUST implement the Git reftable binary specification (`REFT` header v1/v2, `r`/`i`/`o`/`g`
  blocks, prefix compression + sorted 3-byte `restart_offset` binary search, `0x0`–`0x3` ref
  values, CRC-32 footer, `tables.list` + `tables.list.lock`, backward-compat stub `HEAD` and
  `refs/heads`).
- MUST implement geometric factor-2 compaction and atomic ref transactions across immutable tables.
- MUST implement `arks refs migrate --ref-format=reftable|files [--no-reflog] [--dry-run]` and
  `arks refs verify [--strict]`.

### 2.7 E6 & E7 — Native Git LFS Client and Server
- MUST implement canonical Git LFS pointer v1 (`version`, `oid sha256:`, `size`), content-addressed
  local store `.git/lfs/objects/{aa}/{bb}/{oid}`, streaming SHA-256 verification, in-process
  `clean`/`smudge`/`filter-process` (pkt-line v2), and Batch API + Basic Transfer adapter.
- `arks serve` (`E7`) MUST expose `/<repo>.git/info/lfs/objects/batch`, object upload/download, and
  verify routes behind existing `arks-server` auth/ACL layers, with `--no-lfs` opt-out.
- Supersedes the C15/C16 LFS trim in [[research:root/20261111035700]] once E6/E7 land.

### 2.8 E8 — `arks serve` Embedded SSR Web UI
- MUST be embedded directly in the `arks` binary (`include_str!` / compile-time templates +
  self-hosted HTMX v2 asset at `/assets/htmx.min.js`, zero external CDN, zero Node.js build step),
  with `--no-web-ui` opt-out, strict HTML escaping, and
  `CSP: default-src 'self'; script-src 'self'` (blocking all inline scripts and external origins).
- Follows the **GitHub/GitLab common interface** (top repository header + clone URL, horizontal
  `Code` / `Commits` / `Branches` / `Tags` tabs, `<details>` branch/tag switcher, breadcrumb bar,
  directories-first file tree, safe server-side README Markdown preview, About sidebar, line-numbered
  blob viewer + `/raw/` download, and colorized unified commit diff) with `hx-boost="true"`
  progressive enhancement backed directly by `arks-repo`.

---

## 3. Out of Scope (Keep Refuse)

- Desktop GUI embeds (`gitk`, `git-gui`, `citool`, `gui.tcl`).
- GitHub-proprietary ecosystem groups (`codespace`, `copilot`, `extension`, `agent-task`, `event*`, `skill`).
- Cloud-sync folder safety (`arks host-env`).
- CELIR object store (withdrawn).
- Absolute SHA-1 collision immunity and classic UNIX double-fork daemonization.

---

## 4. Verification Intent

Each epic E1–E8 requires:
1. Pure unit + integration tests in its target crate(s) passing 3-OS CI (`ubuntu-24.04`, `macos-15`, `windows-2025`, MSRV `1.85`).
2. A dedicated Docker clean-room gate receipt under `.spec/evidence/` comparing against stock Git / `git-lfs` / fixture servers before its status in [[sot:root/production-residual-register]] and `docs/completion.md` is promoted from **Design** to **Claimed** or **Verified**.
