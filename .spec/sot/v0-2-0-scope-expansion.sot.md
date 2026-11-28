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

Version **0.2.0** (**Claimed (Docker clean-room)** — receipt `.spec/evidence/20261128140500-v0-2-0-e1-e8-cleanroom/receipt.json`). Normative specification for expanding `arks` beyond
the `v0.1.x` baseline to remove the top real-world adoption friction points while deepening
multi-agent concurrency and self-host ergonomics.

Technical references and algorithms: [[research:root/20261128092500]].
Executable ladder: [[plan:root/20261128092500]].

> **Honesty notice:** E1–E8 are **Claimed (Docker clean-room)** via the pinned `arks-verify-m1:0.1.0`
> container receipt (`.spec/evidence/20261128140500-v0-2-0-e1-e8-cleanroom/receipt.json`, Rust 1.99.1,
> stock `git 2.39.5`, zero `gix`, zero `celir`). Live `github.com` OAuth/Actions runs remain
> secrets-gated and are not fabricated as live PASS.

---

## 1. Scope and Epics (E1–E8)

| Epic | Wave | Surface | Target crate(s) | Current status | Verification gate |
|------|------|---------|-----------------|----------------|-------------------|
| **E1** | Wave 1 | OAuth 2.0 Device Authorization Grant (`arks auth login --web`) | `arks-hub`, `arks` | **Claimed (Docker)** | Docker fixture OAuth server E2E (RFC 8628 state machine) — PASS |
| **E2** | Wave 1 | GitHub Actions CLI (`run`, `workflow`, `cache`, `secret`, `variable`) + pure-Rust NaCl sealed-box | `arks-hub`, `arks` | **Claimed (Docker)** | Docker fixture REST API E2E + libsodium sealed-box KAT — PASS |
| **E3** | Wave 1 | Worktree-free history replay (`arks replay`) | `arks-repo`, `arks` | **Claimed (Docker)** | Docker clean-room interop (`arks replay --ref-action=print` → stock `git update-ref --stdin` & `fsck`) — PASS |
| **E4** | Wave 1 | `arks maintenance` + `arks subtree` + 5-command Partial corpus promotion (`fast-import`, `index-pack`, `http-fetch`, `http-push`, `help`) | `arks-repo`, `arks-pack`, `arks` | **Claimed (Docker)** | Docker clean-room `maintenance_subtree_cli` + live `maintenance run --task=gc` & stock `git fsck` — PASS |
| **E5** | Wave 2 | Git Reftable backend (`.git/reftable/`, `extensions.refStorage = reftable`, `arks refs migrate`/`verify`) | `arks-refs`, `arks-repo`, `arks` | **Claimed (Docker)** | Docker clean-room `files → reftable → verify --strict → files` round-trip verified by stock `git show-ref` & `fsck` — PASS |
| **E6** | Wave 2 | Native Git LFS client (`arks lfs` + in-process `clean`/`smudge`/`filter-process` + Batch/Basic transfer) | `arks-remote`, `arks-repo`, `arks` | **Claimed (Docker)** | Docker clean-room `lfs_cli` E2E — PASS |
| **E7** | Wave 2 | Self-host LFS endpoint (`arks serve` `info/lfs` Batch + object store) | `arks-server` | **Claimed (Docker)** | Docker clean-room `serve_lfs_cli` E2E — PASS |
| **E8** | Wave 3 | Single-binary embedded SSR web UI & interactive Forge for `arks serve` (`--no-web-ui` opt-out) | `arks-server` | **Claimed (Docker)** | Docker clean-room `serve_web_ui_cli` + live `arks serve` CSP/Forge merge + stock `git clone` coexistence — PASS |

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
  self-hosted assets at `/assets/htmx.min.js`, `/assets/diff2html.min.js`,
  `/assets/diff2html.min.css`, `/assets/app.js`, `/favicon.svg`; zero external CDN, zero Node.js
  build step), with `--no-web-ui` opt-out, strict HTML escaping, and
  `CSP: default-src 'self'; script-src 'self'` (blocking all inline scripts and external origins).
- Follows the **GitHub/GitLab common forge interface** backed directly by `arks-repo`:
  - Full 9-tab navigation (`Code`, `Issues`, `Pull Requests`, `Actions`, `Projects`, `Wiki`,
    `Security`, `Insights`, `Settings`), global `Cmd+K` / `/` command palette, and Dark/Light theme.
  - 3-pane `Code` view (left file tree sidebar, directories-first explorer, `<details>` branch/tag
    switcher, `arks`/`git` clone copy, safe server-side GFM README preview, About + Language bar
    sidebar), HTMX live file search (`/search`), HTMX commit pagination (`/commits/<rev>?after=…`),
    pure-Rust server-side syntax-highlighted `/blob/` + LFS pointer card + `/raw/` download,
    `/blame/<rev>/<path>`, and dual-line-number Unified/Split `/commit/<oid>` +
    `/compare/<base>..<head>` diffs enhanced with vendored `diff2html`.
  - Self-hosted forge tabs (`Issues`, `Pull Requests`, `Actions`, `Projects`, `Wiki`, `Security`,
    `Insights`, `Settings`) surface live repository/forge data where implemented and honestly label
    not-yet-wired workflow/board backends without fabricating Claimed status.

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
