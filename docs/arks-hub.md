---
title: Forge client (flat on arks)
description: gh-shaped forge client flattened onto arks — same binary; host via arks serve
nav_order: 45
sections:
  - what
  - alias
  - vs-core
  - commands
  - honesty
  - {c: cta, name: next}
---

# Forge client (flat on arks)

Forge collaboration (pull requests, issues, releases, Actions runs, and related host APIs) lives as **top-level** commands on the **same** `arks` CLI — a **`gh`-like client** beside git porcelain. There is **no** separate `arks-hub` release binary. Nested `arks hub …` is **transitional** only (forwards to flat leaves or serve).

You do **not** need forge client features for everyday Git work (`git`→`arks`). To **host** a local GitHub-class forge, use **`arks serve`** ([Self-host](../self-host/)) — **not** `arks hub serve` as the primary story.

## What it is

- Part of the **single** published CLI: **`arks`**
- Internal library crate may still be named `arks-hub`; users type **`arks pr …`**, **`arks issue …`**, **`arks auth …`**, …
- Command names track common **`gh`** muscle memory (`arks pr merge` ↔ `gh pr merge` for mapped commands)
- Talks to the host with a **native HTTPS API client** — it does **not** run `gh` or `git` underneath
- **Client only** for the product story (no web UI in v1). Local forge **server** belongs under `arks serve`

## Alias sketch

```bash
# Git porcelain (required product story):
alias git=arks

# Forge peer personality (optional):
alias gh=arks
# or flat on bare arks:
arks pr list
arks issue create
arks auth status
```

Example: `gh pr merge 12` muscle memory maps to `arks pr merge 12` (host API). That is **not** the same as `arks merge` (Git merge).

Personality details: [CLI personality](../cli-personality/).

## Versus core Git commands

| | **Git on `arks`** | **Forge client on `arks`** |
|--|-------------------|----------------------------|
| Job | Git CLI, remotes, leases | PRs, issues, releases, Actions, forge auth |
| Familiar peer | `git` | `gh` (mapped rows → flat `arks …`) |
| Required for Git DoD? | Yes | **No** |
| Separate release binary? | — | **No** (same `arks`) |
| Host process | — | **`arks serve`** (client is not the host) |

**Overlap nouns (design):** under personality `arks`, `status` / `config` / `repo` use git-first parse + unified Git+Forge output/exit — not hard-renames as the primary story. Optional `inbox` / `forge-config` MAY exist as demoted aliases only. `arks auth` ≠ `arks credential*`.

## Command groups (design)

Surveyed against `gh` help; phases live in Specfold. **v1-shaped** flat groups include:

`auth` · `pr` · `issue` · `release` · `repo` · `label` · `run` · `workflow` · `search` · `api` · `org` · `alias` · `completion`

(Plus other matrix rows as they leave refuse stubs.)

Normative matrix: [arks hub gh command matrix](https://github.com/modootoday/arks/blob/main/.spec/sot/arks-hub-gh-command-matrix.sot.md) · [CSV](https://github.com/modootoday/arks/blob/main/.spec/sot/arks-hub-gh-command-matrix.csv) (logical pillar; argv is flat).

## Honesty

Until a feature has implementation evidence, treat forge client capability as **design / Partial**, not **Claimed**. Tip honesty: many flat leaves are **Partial** against local forge API; **HTTPS remote** via ureq Partial (timeouts/redirect cap/transport retry); `arks hub smoke` is an **HTTPS forge e2e harness Partial** (smoke against `https://` or documented Missing on local http — never spawns `gh`; ≠ Claimed forge remote e2e). `arks hub smoke --out-refuse` is a **residual OUT harness Partial** (proves `extension`/`copilot` stay exit-2 refuse). OUT groups (`extension` / `copilot`) are **explicit refuse**. Core Git symmetry does not depend on forge client. Do not expect a separate crates.io `arks-hub` binary. Transitional `arks hub serve` does **not** invent forge-in-serve Claimed — see [Self-host](../self-host/) and [Completeness](../completion/).

## Next

[Self-host](../self-host/) [CLI personality](../cli-personality/) [Agent](../agent/) [Limits](../limits/) [Git vs arks](../git-vs-arks/) [Getting started](../getting-started/)
