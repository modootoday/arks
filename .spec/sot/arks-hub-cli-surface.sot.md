---
id: sot:root/arks-hub-cli-surface
title: arks hub CLI surface
status: active
created: 2026-11-10T19:01:55Z
updated: 2026-11-11T00:40:00Z
tags: [arks-hub, arks-forge-client, cli, gh-symmetry, flat-cli, pr, issue, auth, repo, release, run, workflow, label, native-api, single-release, client-only, overlap]
verify: manual
rel:
  depends_on:
    - sot:root/arks-hub-product-intent
    - sot:root/arks-hub-gh-command-matrix
    - sot:root/arks-cli-personality-factory
    - sot:root/arks-partial-residual-wave
    - research:root/20261111093543
    - sot:root/product-naming
    - sot:root/cli-surface
    - sot:root/remote-host-compatibility-surface
    - sot:root/public-surface
    - plan:root/20261111040155
  evidence: []
---

# arks hub CLI surface

Version **0.4.1**. Normative **CLI surface** (document revision — **not** a product release) for **flat forge client** leaves on the **single** published binary **`arks`**. Intent: [[sot:root/arks-hub-product-intent]] (**client only**, flat-first). **Authoritative inventory / phases:** [[sot:root/arks-hub-gh-command-matrix]] (+ CSV; logical `gh`↔`arks` pillar — argv need not nest under `hub`). Core git names stay as ordinary `arks <git-cmd>` ([[sot:root/cli-surface]]). Self-host **server**: [[sot:root/arks-self-host-server]] (`arks serve`). Design / Partial until Claimed.

**IW-HUB-FLAT** ([[sot:root/arks-partial-residual-wave]] + [[sot:root/arks-verify-docker-e2e-claim-wave]]): Claimed **subset** create/list/view leaves evidenced ([[evidence:root/20261111011539-hub-flat-subset]]); remaining v1 Missing stay open. **B3** Claimed remote happy path **MUST NOT** invent full-matrix leaf Claimed. Flat client argv ≠ `arks serve` forge host (IW-FORGE-SERVE Claimed separately).

## Rule

### Identity (single CLI — merged surface)

1. **Published binary is `arks` only.** There is no separate `arks-hub` release binary.
2. Forge UX is **top-level** `arks <gh-leaf> …` (e.g. `arks pr merge`). Internal crate MAY stay `arks-hub`. Nested **`arks hub …` is transitional forward-only**.
3. **Name match vs `gh`:** For every CSV row with target ≠ `OUT`, prefer the same leaf at top level: `arks pr merge` ≡ muscle memory of `gh pr merge`, subject to [[sot:root/arks-hub-product-intent]] collision policy.
4. **Global flags (intent, forge leaves):** `--help`, `--version`, `--host <api-base>`, `--repo <owner/name>`, `--hostname` (compat synonym when claimed). Infer repo from local config/remotes via **in-process** `.git` parse — **never** `git`/`gh` spawn.
5. **Auth material:** env (`ARKS_HUB_TOKEN` / documented host-compatible names) + forge-client config/keychain via native APIs — **never** `gh auth` subprocess. `auth setup-git` is **out-of-scope**. **`arks auth` ≠ `arks credential*`.**
6. **Exit codes:** 0 success; non-zero on failure/refuse. Prefer gh-like classes when claimed.

### v1 flat leaves (must track gh names)

Authoritative leaves + flags depth: matrix CSV. Invoked as `arks <group> …` (not `arks hub <group>`). Summary:

| Group | Role | Notes |
|-------|------|-------|
| `auth` | login/logout/status/token/refresh (+ later switch) | No `setup-git`; ≠ credential helpers |
| `pr` | create/list/view/diff/edit/close/reopen/comment/merge/ready/review/checks/… + `checkout` | `co` alias |
| `issue` | create/list/view/edit/close/reopen/comment/… | artifact/develop = later |
| `release` | create/list/view/upload/download/edit/delete/… | verify* = later |
| `repo` | forge create/list/view/fork/… **and** git skew forms | **Overlap** under `arks`: git0→gh1 parse; unified output ([[sot:root/arks-cli-personality-factory]]) |
| `label` | list/create/edit/delete/clone | |
| `run` | list/view/watch/cancel/rerun/download/delete | Actions runs |
| `workflow` | list/view/run/enable/disable | |
| `search` | issues/prs/repos (+ later code/commits) | |
| `status` | git status **and** gh-status/inbox semantics | **Overlap** primary noun; optional demoted `inbox` alias only |
| `config` | git-config **and** forge-client settings | **Overlap** primary noun; optional demoted `forge-config` alias only |
| `inbox` | optional alias → overlap `status` forge facet | **Demoted**; not primary |
| `forge-config` | optional alias → overlap `config` forge facet | **Demoted**; not primary |
| `api` | raw authenticated HTTP | native client |
| `org` | list | |
| `alias` / `completion` | forge/CLI UX parity with gh | completion MAY cover whole `arks` |

### Later / out-of-scope / transitional

7. **Later:** `project`, `discussion`, `gist`, `browse`, `secret`, `variable`, `cache`, `ruleset`, `attestation`, `ssh-key`, `gpg-key`, `codespace`, richer `repo`/`issue` previews — keep **gh names** when mapped and collision-safe.
8. **Out-of-scope:** `extension`, `copilot`, `skill`, `agent-task`, `licenses`, `preview`, `auth setup-git` — see matrix.
9. **Transitional:** `arks hub …` forwards to flat leaves; `arks hub serve` / `hub listen` → `arks serve` forge module ([[sot:root/arks-self-host-server]]). Optional top-level `milestone`; `pr review` remains v1.

### Name hygiene vs core Git commands

| User types | Personality / path | Behavior |
|------------|--------------------|----------|
| `gh pr merge` / `arks pr merge` | exclusive forge leaf | host API |
| `arks merge` / `git merge` | exclusive Git | Git merge |
| `gh pr checkout` / `arks pr checkout` (+ `co`) | exclusive forge | PR checkout; `arks checkout` remains Git refs |
| `arks status` / `arks config` / `arks repo` | **overlap** under personality `arks` | git0→gh1 parse; **unified** stdout; unified fail-closed exit ([[sot:root/arks-cli-personality-factory]]) |
| `alias git=arks` / `ARKS_PERSONALITY=git` + `status` | git personality | peer-pure git stdout + git exit (CI drop-in) |
| `alias gh=arks` / `ARKS_PERSONALITY=gh` + `status`/`config` | gh personality | peer-pure gh stdout + gh exit (CI drop-in) |

Optional shell functions MAY map `gh`→`arks` for convenience; they are **not** a second product binary.

### Cross-OS

10. Windows / macOS / Linux same argv. Config under XDG or `%APPDATA%\arks\` hub section (or documented hub subdir). UTF-8 messages. Survey refresh may run on any OS that ships the pinned `gh`.

## Why

One published CLI keeps release honest. `arks hub` preserves `gh` muscle memory without a `gh` subprocess or a second crates.io bin.

## Verification

1. `arks hub --help` lists implemented groups only until Claimed; refuse others.
2. Matrix coverage check vs pinned `gh help reference`.
3. No product-path `gh`/`git` spawn.
4. No docs claiming a separate `arks-hub` release binary.
5. **IW-HUB-FLAT:** B3 HTTPS remote Claimed receipts **MUST NOT** flip flat-leaf Claimed; hub matrix STRICT + peer-faithful bodies required ([[research:root/20261111093543]]).