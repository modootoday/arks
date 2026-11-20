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

Forge collaboration (pull requests, issues, reviews, labels, milestones, releases with assets, and forge auth) lives as **top-level** commands on the **same** `arks` CLI — a **`gh`-like client** beside git porcelain. There is **no** separate `arks-hub` release binary. Nested `arks hub …` is **transitional** only (forwards to flat leaves or serve).

You do **not** need forge client features for everyday Git work (`git`→`arks`). To **host** the arks forge, use **`arks serve`** ([Self-host](../self-host/)) — **not** `arks hub serve` as the primary story.

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
| Job | Git CLI, remotes, leases | PRs, issues, releases, forge auth |
| Familiar peer | `git` | `gh` (mapped rows → flat `arks …`) |
| Required for Git DoD? | Yes | **No** |
| Separate release binary? | — | **No** (same `arks`) |
| Host process | — | **`arks serve`** (client is not the host) |

**Overlap nouns (design):** under personality `arks`, `status` / `config` / `repo` use git-first parse + unified Git+Forge output/exit — not hard-renames as the primary story. Optional `inbox` / `forge-config` MAY exist as demoted aliases only. `arks auth` ≠ `arks credential*`.

## Command groups (design)

Surveyed against `gh` help; phases live in Specfold. **v1-shaped** flat groups include:

`auth` · `pr` · `issue` · `release` · `repo` · `label` · `search` · `api` · `org` · `alias` · `completion` · `config` · `status` · `browse` · `ruleset` / `rs`

**Flags follow `gh`.** Every leaf parses `gh`'s flags (long and short spellings, `--flag=value`, bundled short flags) and refuses unknown flags; `--json` / `--jq` / `--template` use `gh`'s field names. Output is `gh`'s piped (non-TTY) form. `--web` prints the URL instead of opening a browser. Commands that would run Git locally — `pr checkout`, `repo clone`, `repo fork --clone`, `repo create --clone`, a local `repo sync` — print the `arks` commands to run instead; `arks` never spawns `git` or `gh`.

**Features the `arks serve` forge provides for them:**

| Feature | Commands |
|---------|----------|
| Repository content (each forge repository has a Git directory under `--forge-git-root`, default `<forge-data>/git`) | `repo create --add-readme/--gitignore/--license`, `repo read-dir` / `read-file`, `pr diff`, real `pr merge` (merge / squash / rebase), `pr update-branch`, `pr revert`, `repo fork`, `repo sync`, `release download --archive` |
| Commit statuses | `pr checks` (post a status with `arks api -X POST repos/OWNER/REPO/statuses/SHA -f state=success -f context=ci`; exit 8 while pending, 1 on failure) |
| Rulesets | `ruleset list` / `view` / `check`; create one with `arks api -X POST repos/OWNER/REPO/rulesets --input ruleset.json`. Active rulesets block `pr merge` until required reviews and status checks pass (pushes are not checked) |
| Template catalogs | `repo gitignore list|view`, `repo license list|view` (arks's own short templates; licenses point to the SPDX text) |
| Search | `search issues|prs|repos|code|commits` with `gh`'s qualifiers |
| Issue extras | `issue develop` (linked branches), `issue artifact create|list|view|edit|download|delete` |
| Accounts | `auth login` (several accounts per host), `auth switch`, `auth status --json hosts` |

**arks-owned forge features** (argv symmetric with `gh`; arks defines storage and semantics in the `arks serve` forge):

| Group | Leaves |
|-------|--------|
| `gist` | `create` · `list` · `view` · `edit` · `delete` · `rename` · `clone` |
| `discussion` | `create` · `list` · `view` · `edit` · `comment` |
| `project` | `list` · `view` · `create` · `edit` · `delete` · `close` · `copy` · `link` · `unlink` · `mark-template` · `field-create` · `field-delete` · `field-list` · `item-add` · `item-archive` · `item-create` · `item-delete` · `item-edit` · `item-list` |
| `ssh-key` / `gpg-key` | `list` · `add` · `delete` |
| `repo autolink` | `create` · `list` · `view` · `delete` |
| `repo deploy-key` | `add` · `list` · `delete` |

Notes: a forge gist is a file map without git history, so `arks gist clone` writes the files into a directory and prints an `arks init` recipe (it never runs `git`). `gist edit` and `discussion comment` take a file or flag value instead of opening an editor. SSH and OpenPGP keys are checked for a well-formed public key (fingerprints follow OpenSSH and RFC 9580); private key material is refused, and OpenPGP key expiry is not evaluated. Projects have no teams or iteration fields. `arks repo autolink` / `arks repo deploy-key` reach the forge directly; on the Git side `arks repo` only owns `info` / `structure`. These leaves are tested on the host against a local forge (**Partial**, host-advisory) — no Docker receipt yet.

arks is an independent forge, not a port of GitHub. `gh` supplies command names only. GitHub-platform-only services are **out of scope** and refuse with exit 2: Actions (`run`, `workflow`, `cache`, `secret`, `variable`), `attestation`, `codespace`, and `release verify` / `release verify-asset`. Rulesets (`ruleset` / `rs`), `repo gitignore` / `repo license` and `pr checks` are implemented as arks-owned forge features (see above). Decision record: [ADR 20261119230000](https://github.com/modootoday/arks/blob/main/.spec/adr/20261119230000-arks-forge-independent-compatible-subset.adr.md).

Normative matrix: [arks hub gh command matrix](https://github.com/modootoday/arks/blob/main/.spec/sot/arks-hub-gh-command-matrix.sot.md) · [CSV](https://github.com/modootoday/arks/blob/main/.spec/sot/arks-hub-gh-command-matrix.csv) (logical pillar; argv is flat).

## Honesty

Until a feature has implementation evidence, treat forge client capability as **design / Partial**, not **Claimed**. Tip honesty: flat create/list/view **subset** + expand + **mutate** leaves are **Claimed** where evidenced ([[evidence:root/20261111011539-hub-flat-subset]], [[evidence:root/20261111020350-hub-full-expand]], [[evidence:root/20261111025142-hub-mutate-subset]]); assets / `pr checkout` / UX Claimed under in-scope leftovers; **IW-HUB-STUBS** Claimed subset (`pr status|ready`, `issue status|delete`, `repo edit|set-default`, `api`, `status`) with Docker receipt — lock/diff/checks/OAuth/fork Stay Partial (CSV Missing=0). **B3** HTTPS forge remote e2e is **Claimed** ([[evidence:root/20261110233655-claimed-hub-https]]). `arks hub smoke` remains an **HTTPS forge smoke harness Partial** (never spawns `gh`; smoke ≠ B3). `arks hub smoke --local-tls` is a **self-signed local HTTPS + client path Partial** (≠ **B9** product in-process TLS Claimed — see [Self-host](../self-host/) / [[evidence:root/20261110233652-claimed-tls]]). `arks hub smoke --out-refuse` is a **residual OUT harness Partial** (proves every OUT group stays exit-2 refuse). OUT groups (`extension` / `copilot` and the GitHub-platform-only groups above) are **explicit refuse**. The gh-flag rework of the flat leaves is tested on the host against a local forge (**Partial**, host-advisory; no Docker receipt yet). Core Git symmetry does not depend on forge client. Do not expect a separate crates.io `arks-hub` binary. Transitional `arks hub serve` does **not** invent forge-in-serve Claimed — residual SoT: [arks-claimed-residual-gates](https://github.com/modootoday/arks/blob/main/.spec/sot/arks-claimed-residual-gates.sot.md). See [Completeness](../completion/).

## Next

[Self-host](../self-host/) [CLI personality](../cli-personality/) [Agent](../agent/) [Limits](../limits/) [Git vs arks](../git-vs-arks/) [Getting started](../getting-started/)
