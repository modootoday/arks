---
id: research:root/20261110095142
title: Product completion goal ladder (post-PF5 8/8)
status: active
created: 2026-11-10T09:51:42Z
updated: 2026-11-10T19:00:00Z
tags:
  [
    product-completion,
    residual,
    public-surface,
    release-readiness,
    git-asymmetry,
    pf5,
    r1-r6,
  ]
rel:
  depends_on:
    - sot:root/production-residual-register
    - sot:root/program-roadmap
    - sot:root/product-intent
    - sot:root/public-surface
    - sot:root/arks-remote
    - sot:root/arks-server
    - sot:root/cli-surface
    - sot:root/agent-dev-readiness
    - sot:root/clean-room-journey-compare
    - research:root/20261110085731
    - research:root/20261108071916
    - adr:root/20261110190000
    - evidence:root/20261110190000
---

# Product completion goal ladder (post-PF5 8/8)

## Question

After PF5 **Claimed 8/8** ([[evidence:root/20261110142500]]) and residual §F **38/38** terminal with reopen-only **R1–R6**, what ordered **product-completion** goals remain that are not “invented Opens,” spanning Git asymmetry, public-surface / release readiness, and Specfold DoD honesty?

## Method

1. Read [[sot:root/production-residual-register]] §F + post-terminal next-queue; [[sot:root/program-roadmap]] M0–M26 exits; [[sot:root/arks-remote]] / [[sot:root/arks-server]] claimed vs not-claimed transports.
2. Audit [[sot:root/public-surface]] release checklist vs tree (`README`, `docs/`, `SECURITY.md`, `CONTRIBUTING.md`, release workflows research [[research:root/20261110085731]]).
3. Spot-check human Pages honesty against carrying PF5 evidence (especially `docs/limits.md` / performance copy).
4. Rank by: honesty breakage first → highest-value Git asymmetry reopen → release/docs DoD → secrets-gated / process-only.

## Findings

### Honesty baseline (tip)

| Item | State |
|------|-------|
| Tip residual / PF5 | §F terminal; PF5 **8/8** primary [[evidence:root/20261110142500]] @ ~`79fc9bf` |
| Bare development Opens | **0** |
| R1–R6 | Reopen-only (not bare Opens); **R1** + **R2** Claimed (host wire) |
| Host CI | Advisory vs Docker gates; tip green does not invent Verified |

### Ordered product-completion ladder

| # | Goal | Class | Disposition / reopen | Why now |
|---|------|-------|----------------------|---------|
| **L0** | Public honesty parity (Pages/README vs PF5 8/8 + residual) | Docs / DoD | **Done** | Stale “Clone HTTP aggressive_na” fixed |
| **L1** | Root `CONTRIBUTING.md` thin pointer → `docs/contribute.md` | Public-surface release | **Done** | Pages contribute exists; root CONTRIBUTING landed |
| **L2** | Client **`git://`** clone/fetch/push (**R1**) | Git asymmetry | **Done (Claimed host wire)** | [[sot:root/arks-remote]] v0.5.1+ |
| **L3** | Dumb HTTP **push** (**R2**) | Git asymmetry | **Done (Claimed host wire)** | [[adr:root/20261110190000]] + [[evidence:root/20261110190000]] |
| **L4** | Release-channel readiness (OIDC registries / checksum Releases) | Release | Follow [[research:root/20261110085731]] / `release.yml`; no invented publish PASS | Workflows landed; operator Environments / tag publish remain ops |
| **L5** | Live forge scale SLO / G8 live PASS (**R5**) | Secrets-gated | Secrets + receipt only | Harness Claimed; never invent latency |
| **L6** | Partial-clone filter/promisor (**R3**) | N/A | Dedicated SoT amend beyond `--depth` | Honest N/A |
| **L7** | OpenPGP / X.509 / `tag -s` (**R4**) | N/A | Dedicated crypto harness SoT | Honest N/A |
| **L8** | Clean-room Git pin advance (**R6**) | Process | Deliberate pin bump + G14-class re-prove | Pin **2.39.5** process-closed |
| **L9** | crates.io / docs.rs publish timing | Owner policy | Closed default in [[sot:root/public-surface]] | Not a residual Open |

### Non-goals (do not treat as incomplete product)

- Restarting sole G19 remasure without honesty breakage.
- Relitigating §F Claimed / Refuse / N/A rows as bare Opens.
- Host cargo-test or Docker OOM release-build loops as “Done.”
- Inventing Claimed/PASS / forge OAuth / CELIR ObjectStore.

### Execution note (this round)

**L0–L3 Done.** Next recommended: **L4** release-channel ops (Environments / tag dry-run docs+wiring; no invented publish PASS). L5+ stay queued (secrets / N/A / process).

## Verification

1. Ladder cited from residual register / AGENTS progressive-disclosure pointer (no second SoT).
2. Each ladder row maps to an owning SoT/ADR/research cite; no invented Git semantics.
3. When L2 ships: [[sot:root/arks-remote]] flips `git://` from Not claimed → Claimed with wire tests; residual R1 disposition updates; `spec check` clean.
4. When L3 ships: [[adr:root/20261110190000]] + dumb WebDAV client/server + residual R2 Claimed + [[evidence:root/20261110190000]]; `spec check` clean.
