---
title: CLI personality
description: Single binary arks — git, gh, and arks personalities
nav_order: 35
sections:
  - what
  - resolve
  - overlap
  - honesty
  - {c: cta, name: next}
---

# CLI personality

The published binary is **`arks` only**. Drop-in for stock tools is **personality dispatch** — not a second crates.io binary named `git` or `gh`.

## Personalities

| Personality | When | Behavior |
|-------------|------|----------|
| **git** | `alias git=arks`, basename `git`, or `ARKS_PERSONALITY=git` | Peer-pure Git argv, stdout shape, and exit-code semantics |
| **gh** | `alias gh=arks`, basename `gh`, or `ARKS_PERSONALITY=gh` | Peer-pure forge-client argv, stdout, and exit codes |
| **arks** (default) | Bare `arks` | **Git-first** merged surface: porcelain + flat forge leaves; unified overlap output/exit where designed |

Resolution order (first wins): explicit `ARKS_PERSONALITY` → argv0 / shim basename → default `arks`.

## What each personality owns

- **git** — no forge unified sections required; everyday status/config/porcelain stay Git.
- **gh** — surveyed `gh` leaf grammar only; no Git porcelain unless that `gh` leaf already owns the noun.
- **arks** — flat forge leaves (`pr`, `issue`, `auth`, `release`, …) sit beside Git; nested `arks hub …` is **transitional** only. Hosting is **`arks serve`** (not hub). Daemon is **`arks daemon`**. Optional **`arks agent …`**.

CI and scripts that today call `git` / `gh` should use the **peer** personalities so they receive peer-native stdout and exit codes. Unified `arks` is for humans and arks-native automation that **opts into** merged semantics (or forces a single peer via flags such as `--git-only` / `--forge-only` / `--porcelain` — design names; Partial until Claimed).

## Overlap leaves (`status`, `config`, `repo`, …)

Under personality **arks**:

1. **Parse** Git grammar first; fall through to forge grammar if not a valid/applicable Git form.
2. **Output** a unified Git + Forge view by default (not “pick one peer only”).
3. **Exit** fail-closed: non-zero if either enabled layer fails (unless single-peer flags isolate one peer).

Hard-renames such as `inbox` / `forge-config` are **demoted optional aliases** only — not the taught primary nouns.

## Honesty

Personality dispatch, Registry / OverlapFacade, and unified exit tables are **design / Partial** until clean-room evidence says otherwise. This page does **not** invent Claimed peer drop-in completeness. Product paths never spawn `git` or `gh`.

Owning SoTs: [personality factory](https://github.com/modootoday/arks/blob/main/.spec/sot/arks-cli-personality-factory.sot.md), [output/exit contract](https://github.com/modootoday/arks/blob/main/.spec/sot/arks-cli-output-exit-contract.sot.md).

## Next

[Alias git → arks](../alias-git-arks/) [Completeness](../completion/) [Forge client](../arks-hub/) [Getting started](../getting-started/)
