---
title: Forge client (flat on arks)
description: gh-shaped forge client flattened onto arks — same binary as git; host via arks serve
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

Forge collaboration (pull requests, issues, releases, Actions runs, and related host APIs) lives as **top-level** commands on the **same** `arks` CLI — a **`gh`-like client** merged with git porcelain. There is **no** separate `arks-hub` release binary. Nested `arks hub …` is transitional only.

You do **not** need forge client features for everyday Git work (`git`→`arks`). To **host** a local GitHub-class forge, use **`arks serve`** ([Self-host](../self-host/)).

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

# Forge UX (when implemented) — same binary, flat:
arks pr list
arks issue create
arks auth status
```

Optional: a personal shell function can forward `gh …` to `arks …`. That is convenience only — **not** a second product release.

Example: `gh pr merge 12` muscle memory maps to `arks pr merge 12` (host API). That is **not** the same as `arks merge` (Git merge).

## Versus core Git commands

| | **Git on `arks`** | **Forge client on `arks`** |
|--|-------------------|----------------------------|
| Job | Git CLI, remotes, `arks daemon`, leases | PRs, issues, releases, Actions, forge auth |
| Familiar peer | `git` | `gh` (mapped rows → flat `arks …`) |
| Required for Git DoD? | Yes | **No** |
| Separate release binary? | — | **No** (same `arks`) |

**Collisions (design):** `arks status` / `arks config` stay **git**. Forge `gh status` → `arks inbox`; forge `gh config` → `arks forge-config`. `arks repo` uses subcommand dispatch (git skew `info` vs forge create/list/…). `arks auth` ≠ `arks credential*`.

## Command groups (design)

Surveyed against `gh` help; phases live in Specfold. **v1-shaped** flat groups include:

`auth` · `pr` · `issue` · `release` · `repo` · `label` · `run` · `workflow` · `search` · `inbox` · `api` · `org` · `forge-config` · `alias` · `completion`

Normative matrix: [arks hub gh command matrix](https://github.com/modootoday/arks/blob/main/.spec/sot/arks-hub-gh-command-matrix.sot.md) · [CSV](https://github.com/modootoday/arks/blob/main/.spec/sot/arks-hub-gh-command-matrix.csv) (logical pillar; argv is flat).

## Honesty

Until a feature has implementation evidence, treat forge client capability as **design intent**, not **Claimed**. Core Git symmetry does not depend on forge client. Do not expect a separate crates.io `arks-hub` binary.

## Next

[Self-host](../self-host/) · [Limits](../limits/) · [Git vs arks](../git-vs-arks/) · [Getting started](../getting-started/)
