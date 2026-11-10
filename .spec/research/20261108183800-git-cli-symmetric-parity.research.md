---
id: research:root/20261108183800
title: Git CLI symmetric parity
status: active
created: 2026-11-08T18:38:00Z
updated: 2026-11-08T18:46:00Z
tags: [git, cli, porcelain, parity, research, alias, dot-git]
rel:
  depends_on:
    - sot:root/cli-surface
    - sot:root/git-compat-surface
    - sot:root/program-roadmap
    - research:root/20261108071859
    - sot:root/product-verification-clean-room
---

# Git CLI symmetric parity

Design research supporting the raise of arks **compatibility** to include **complete symmetric support with the stock `git` CLI** (porcelain/plumbing argv parity against a pinned Git). Normative rules live in [[sot:root/cli-surface]] and [[sot:root/program-roadmap]]. This note records primary sources, inventory method, and residual opens — **no invented Git semantics**.

## Primary sources (cite; do not paste)

| Source | URL | Scope used |
|--------|-----|------------|
| git(1) | https://git-scm.com/docs/git | Porcelain vs plumbing taxonomy; main porcelain / ancillary / plumbing command lists; global options |
| gitcli(7) | https://git-scm.com/docs/gitcli | Command-line syntax conventions |
| gitglossary(7) | https://git-scm.com/docs/gitglossary | Definitions of *plumbing* and *porcelain* |
| Per-command manuals | https://git-scm.com/docs/git-&lt;cmd&gt; | Option contracts for flag-parity classification |
| Formats / protocols catalog | [[research:root/20261108071859]] | Byte/wire semantics backing CLI results |

Secondary prose (e.g. Pro Git “Plumbing and Porcelain”) may orient readers; it **must not** override git(1)/man pages.

## Method

1. Lock clean-room **Git version pin** ([[sot:root/product-verification-clean-room]]).
2. Export the pin’s command list (git(1) sections for that version).
3. Build parity inventory = main porcelain + required ancillary/plumbing − [[sot:root/cli-surface]] exclusions (GUI, scalar, LFS, forge APIs, server-role, lease/CELIR admin).
4. For each inventory command, classify every documented option: implement / refuse / exclude.
5. Verify on-disk and behavioral outcomes with stock Git as peer oracle in clean-room (Gate IDs G6, G12–G14, remotes G7+).

## Compatibility layers

| Layer | Meaning | Owner |
|-------|---------|-------|
| Format / protocol / **same `.git`** | Interchangeable repos on stock Git layout | [[sot:root/git-compat-surface]] |
| CLI argv / **alias drop-in** | Symmetric `arks` ↔ `git`; `alias git=arks` muscle memory | [[sot:root/cli-surface]] |
| Production | Same `.git` + inventory + alias-ready + remotes + engine | [[sot:root/program-roadmap]] M9 |

Former “Partial = formats + CLI subset; full porcelain deferred” product bar is **superseded**. Phased **delivery** remains; **end-state** does not defer porcelain or alias drop-in.

## Residual opens

- Exact Pb/Pc command partition tables (milestone plans). **Pa frozen** in [[plan:root/20261108183810]] / `verify/matrices/matrix-m7.md`.
- Plumbing minimum set for scripting without gaps.
- Pin-advance gap-audit process detail when clean-room Git upgrades.
- Credential helper depth; askpass.
- Rebase-form `pull` deferred to Wave Pb; M7 claims FF-only `pull` only.

## Record

| Date (UTC) | Note |
|------------|------|
| 2026-11-08 | Authored with CLI-parity raise; points at git(1)/gitcli/glossary; no Rust. |
