---
id: sot:root/arks-hub-product-intent
title: arks hub product intent
status: active
created: 2026-11-10T19:01:55Z
updated: 2026-11-14T10:30:00Z
tags: [arks-hub, arks-forge-client, forge, collaboration, gh-symmetry, flat-cli, pr, issue, review, release, auth, internal-crate, single-release, native-api, client-only, overlap]
verify: manual
rel:
  depends_on:
    - sot:root/product-intent
    - sot:root/product-naming
    - sot:root/cli-surface
    - sot:root/arks-cli-personality-factory
    - sot:root/remote-host-compatibility-surface
    - sot:root/crate-architecture
    - sot:root/public-surface
    - sot:root/program-roadmap
    - sot:root/arks-hub-cli-surface
    - sot:root/arks-hub-gh-command-matrix
    - sot:root/arks-self-host-server
    - plan:root/20261111040155
  evidence: []
---

# arks forge client product intent (flat CLI)

Version **0.4.0**. Normative **product intent** (document revision — **not** a product release; workspace crate remains `0.0.0`) for forge / host collaboration as **top-level** commands on the **single** published CLI **`arks`** — **merged git + gh client surface**.

**Role:** forge **client** leaves (`arks pr`, `arks issue`, `arks auth`, …) talk to a host API base URL (GitHub.com, another forge, or a local **`arks serve`** forge module). This is **not** the way to start the host — that is `arks serve`.

**Honesty:** Design / Partial for most forge leaves — do not invent Claimed from this SoT alone. **B3** forge HTTPS remote happy path is Claimed under [[sot:root/arks-claimed-residual-gates]] ([[evidence:root/20261110233655-claimed-hub-https]]); hub `--local-tls` smoke alone ≠ B3.

**Advice amend (v0.4.0):** Nested **`arks hub …` as the day-to-day forge noun is demoted.** Prefer flat `gh`-shaped leaves beside git porcelain. Transitional `arks hub <leaf> …` MAY forward to `arks <leaf> …`. Internal crate MAY remain named `arks-hub` (library only).

Host product APIs (remote-host **scope (B)**) as **client surface** live here. Core Git porcelain stays under ordinary `arks` commands ([[sot:root/product-intent]], [[sot:root/cli-surface]]). Self-host **server** umbrella: [[sot:root/arks-self-host-server]].

CLI shape: [[sot:root/arks-hub-cli-surface]]. Exhaustive **`gh` → `arks` (flat) matrix**: [[sot:root/arks-hub-gh-command-matrix]] (logical pillar; argv may omit `hub`). Crate plan: [[plan:root/20261111040155]].

## Rule

### Shipping (normative)

1. **Single release = `arks` only.** Do **not** publish a separate `arks-hub` binary or crates.io product release.
2. Users invoke **flat forge leaves:** `arks pr …`, `arks issue …`, `arks auth …`, … (stock peer muscle memory: **`gh <leaf>`** without a second binary).
3. Implementation MAY live in an **internal** workspace crate **`arks-hub`** (library; name MAY later become `arks-forge`). The `arks` binary depends on it and dispatches **top-level** forge commands. **No** standalone bin. **No** requirement that the CLI group stay named `hub`.
4. Git alias DoD (`git`→`arks`) MUST NOT require forge client features to be Claimed. Forge UX is additive on the same binary.

### Mission

5. Cover forge workflows stock `git` never owned (PR, issue, release, label, Actions run/workflow, search, auth, …) as **top-level** `arks` commands where collision policy allows.
6. Track **`gh` CLI muscle memory** via the matrix: `gh pr merge` ↔ `arks pr merge` for mapped rows ([[sot:root/arks-hub-gh-command-matrix]]). Optional wrappers MAY map `gh`→`arks`. Nested `arks hub …` is **transitional forward-only**.
7. **Native API client — never CLI wrappers.** Talk to hosts over in-process HTTP (Octokit-like). Prefer **`.git`-native** metadata where sensible. **No web UI** required in SoT v1.
8. **Host process is `arks serve`.** Local/self-host forge API belongs under [[sot:root/arks-self-host-server]]. Transitional `arks hub serve` / `hub listen` only.

### Hard non-invocation law (ABSOLUTE)

| Surface | MUST NOT invoke | Allowed |
|---------|-----------------|---------|
| `arks` (incl. flat forge leaves, transitional `arks hub`, `arks daemon`, git porcelain, `arks serve`) | Stock `git` / `gh` on product paths | In-engine Git; native HTTPS for forge client; clean-room/`verify` MAY use `git` outside the product binary |

Matrix rows are **surface** targets for humans. Implementing a row by shelling to `gh` **fails** this SoT.

### Flat merge + collision policy (normative)

9. **Default shape:** flatten surveyed `gh` groups onto top-level `arks` (`arks pr`, `arks issue`, …) beside git porcelain.
10. **Overlap nouns (status / config / repo / …) — superseded hard-rename.** Under personality **`arks`**, do **not** make separate top-level names (`inbox`, `forge-config`) the primary story. Authoritative rule: [[sot:root/arks-cli-personality-factory]] — **input** parse Git grammar rank 0 then gh rank 1; **output** unified Git+Forge view (unless `--git-only` / `--forge-only` / `--porcelain`); **exit** unified fail-closed (non-zero if either layer fails). Optional `inbox` / `forge-config` MAY remain as demoted aliases only.
11. **`credential*` vs `auth`:** git credential helpers remain unrelated to `arks auth` (forge tokens) — not an OverlapFacade noun.
12. **`repo`:** one top-level `arks repo` under OverlapFacade / Registry overlap mark — Git-applicable forms (e.g. skew `info`) rank 0; forge `create`/`list`/`view`/`fork`/… rank 1 fallthrough; unified render when both apply. Do not invent a second top-level `repository` unless dispatch fails review.
13. **Peer personalities:** `ARKS_PERSONALITY=git|gh` / argv0 alias stay peer-pure (no unified output/exit). CI SHOULD use those for drop-in.
14. **Safe exclusive flat leaves (no Git top-level peer):** `auth`, `pr`, `issue`, `release`, `label`, `run`, `workflow`, `search`, `api`, `org`, `alias`, `completion`, `browse`, `gist`, `discussion`, `project`, `ssh-key`, `gpg-key`, `ruleset`, `attestation`, `codespace`, `secret`, `variable`, `cache` (Actions — ≠ `credential-cache`), `co` (alias of `pr checkout` only; document vs user `checkout` aliases).
15. **`arks auth` ≠ credential helpers.** Forge login/token only. Git remote auth stays `credential` / Basic / SSH keys.
16. **Transitional:** `arks hub <args…>` MAY rewrite to flat `arks <args…>` (and `hub serve` → `arks serve` forge module). MUST NOT remain the taught primary surface.

### Goals

| Goal | Normative meaning |
|------|-------------------|
| Single CLI release | Only `arks` is the product binary/release |
| Merged client surface | git porcelain + flat gh-shaped leaves in one argv space |
| Internal crate | `arks-hub` (or later `arks-forge`) library behind flat leaves; not a second published bin |
| gh symmetry | Same leaf names as surveyed `gh` where matrix + collision policy allow: `arks pr merge` |
| Native forge **client** | First verify peer = GitHub-compatible API; `--host` for other hubs / local `arks serve` forge base |
| Host under `serve` | Forge **server** modules land in `arks serve`; client stays flat top-level |
| Honest claims | Design/matrix ≠ Claimed |

### Non-goals

1. Separate `arks-hub` release binary or dual-product marketing.
2. Web UI for forge client v1.
3. Replacing scope **(A)** remotes/server in core Git paths (git wire stays under `arks serve` / `arks-server`).
4. Mirroring gh-only product surfaces marked **out-of-scope** (extensions, Copilot, agent-task/skills, `auth setup-git`, …).
5. Subprocess backends (`gh`/`git`).
6. Inventing Claimed forge features from SoT alone.
7. Treating forge client Claimed as required for `git`→`arks` alias DoD.
8. **`arks hub …` as the primary forge client noun** or **`hub serve` as primary host UX** (both superseded).
9. Treating hard-renames (`inbox` / `forge-config`) as the **primary** overlap story under personality `arks` (superseded by unified overlap in [[sot:root/arks-cli-personality-factory]]).
10. Silently replacing Git `status` / `config` semantics under personality `git`, or omitting Git sections from default unified `arks` overlap output without opt-in single-peer flags.

### Relationship

```text
Published binary:  arks
  personalities: git | gh | arks   ([[sot:root/arks-cli-personality-factory]])
                     ├── git porcelain/plumbing/remotes/lease…   (peer: git)
                     ├── overlap: status/config/repo …           (arks: git0→gh1 + unified I/O/exit)
                     ├── flat forge leaves: pr issue auth api … (peer: gh)
                     ├── arks daemon  → internal arks-daemon     (peer: git-daemon; ≠ product story)
                     ├── arks serve   → hosting umbrella         (git HTTP/SSH + forge API)
                     ├── arks agent   → future (optional)
                     └── transitional: arks hub … → flat / serve
                          └── lib: internal arks-hub (forge client + optional serve forge module)
```

### Success criteria (design phase)

1. Product + CLI + **gh matrix** SoTs active; wording prefers **flat** `arks <gh-leaf>`; `hub` labeled transitional; host = `arks serve`.
2. Matrix covers pinned `gh help reference` leaves; overlap nouns follow [[sot:root/arks-cli-personality-factory]] (unified under `arks`; optional demoted aliases only).
3. Crate graph: internal `arks-hub` lib; `arks` dispatches flat leaves; **no** published second bin. Forge **server** code MAY live in that lib and be called from `arks serve`.
4. Public copy: one binary, merged client; no Claimed without evidence.

## Why

One binary should feel like **git + gh merged**, not a third nested noun (`hub`) on top of an already large CLI. Flat leaves match GitHub-like product unity with `arks serve` as the host. Collision policy keeps git alias DoD honest. Prior “keep `arks hub` as the CLI noun” advice is amended for this intent.

## Verification

1. `spec check` for this SoT and matrix dependents.
2. Product-path spawn of `gh`/`git` fails review.
3. No Claimed forge client features from design-only.
4. Docs teach flat leaves + `arks serve`; `hub` transitional only.
