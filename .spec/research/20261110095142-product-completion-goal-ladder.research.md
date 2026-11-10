---
id: research:root/20261110095142
title: Product completion goal ladder (post-PF5 8/8)
status: active
created: 2026-11-10T09:51:42Z
updated: 2026-11-10T10:45:00Z
tags:
  [
    product-completion,
    residual,
    public-surface,
    release-readiness,
    git-asymmetry,
    pf5,
    r1-r6,
    clean-room-e2e,
    comparison-matrix,
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
| **L4** | Release-channel readiness (OIDC registries / checksum Releases) | Release | **Done (ops dry-run)** — Environments live; `workflow_dispatch` dry-run + `docs/release.md` + `scripts/release-channel-readiness.sh`; **no** invented live publish PASS | Follow [[research:root/20261110085731]] / [[plan:root/20261110085731]]; live tag + trusted publishers remain operator |
| **L5** | Live forge scale SLO / G8 live PASS (**R5**) | Secrets-gated | **Done (ops dry-run / readiness)** — Environment `g8-live` + `g8-forge-scale.yml` dry-run + `scripts/forge-scale-readiness.sh` + `docs/forge-scale.md`; harness Claimed; **no** invented live latency PASS | Secrets + receipt only for any future remasure; never invent latency |
| **L6** | Partial-clone filter/promisor (**R3**) | N/A | **Done (honest N/A)** — SoT remains `--depth` only; CLI refuses `--filter`; **no** filter/promisor Claimed/PASS; reopen = SoT amend beyond `--depth` + wire + G35-class re-verify | Ladder close ≠ product claim |
| **L7** | OpenPGP / X.509 / `tag -s` (**R4**) | N/A | **Done (honest N/A)** — FW-GPGSIG stays SSH commit create/verify only; CLI refuses `tag -s`/`-u`, OpenPGP/`gpg.format=openpgp`, X.509, and `verify-tag`; **no** signed-tag / OpenPGP / X.509 Claimed/PASS; reopen = dedicated crypto harness SoT + wire + G37-class re-verify | Ladder close ≠ product claim |
| **L8** | Clean-room Docker multi-scenario git↔arks e2e | Verify DoD | **In progress** — required core `g14` + `g16` **PASS** on tip (Docker receipts under `verify/gates/*/out/`); expand `g20`/`g21` per `verify/matrices/matrix-l8-e2e.md` until green; halt OOM/stampede; **≠** host-only smoke; **≠** invent PASS | True product-completion bar |
| **L_final** | Finished git↔arks feature comparison matrix | Docs / DoD | **In progress** — public `docs/git-vs-arks.md` + Specfold-backed Claimed/N/A/reopen rows; must stay honest vs residual register | Human deliverable |
| **R6 note** | Clean-room Git pin (**2.39.5**) | Process | Remains **process-closed** ([[sot:root/clean-room-git-pin-advance]]); deliberate pin bump + G14-class re-prove when advancing — **not** the L8 product DoD | Residual reopen-only |
| **L9** | crates.io / docs.rs publish timing | Owner policy | Closed default in [[sot:root/public-surface]] | Not a residual Open |

### Non-goals (do not treat as incomplete product)

- Restarting sole G19 remasure without honesty breakage.
- Relitigating §F Claimed / Refuse / N/A rows as bare Opens.
- Host cargo-test or Docker OOM release-build loops as “Done.”
- Inventing Claimed/PASS / forge OAuth / CELIR ObjectStore.
- Treating clean-room Git pin advance (**R6**) as a substitute for L8 e2e green.

### Execution note (this round)

**L0–L7 Done.** L7 = **honest N/A** for residual **R4**. **L8 in progress:** Docker `g14` PASS + `g16` PASS (19 journeys) on tip; expand `g20`/`g21` per matrix until suite Done. **L_final in progress:** `docs/git-vs-arks.md` drafted + nav linked. **L9** closed. **R6** pin process-closed (not L8).

## Verification

1. Ladder cited from residual register / AGENTS progressive-disclosure pointer (no second SoT).
2. Each ladder row maps to an owning SoT/ADR/research cite; no invented Git semantics.
3. When L2 ships: [[sot:root/arks-remote]] flips `git://` from Not claimed → Claimed with wire tests; residual R1 disposition updates; `spec check` clean.
4. When L3 ships: [[adr:root/20261110190000]] + dumb WebDAV client/server + residual R2 Claimed + [[evidence:root/20261110190000]]; `spec check` clean.
5. When L4 ships: Environments `crates`/`npm`/`pypi`/`rubygems` + `release.yml` dry-run wiring + maintainer docs; **no** Verified registry publish PASS invented; `spec check` clean.
6. When L5 ships: Environment `g8-live` + `g8-forge-scale.yml` dry-run + readiness script + maintainer docs; harness SKIP without secrets; **no** invented live forge latency PASS / G8 remasure Claimed from empty receipts; `spec check` clean.
7. When L6 ships: residual **R3** stays **N/A**; public/`docs/limits.md` keep filter/promisor unclaimed; clone/fetch refuse `--filter` fail-closed; **no** invented filter/promisor Claimed/PASS; `spec check` clean.
8. When L7 ships: residual **R4** stays **N/A**; public/`docs/limits.md` keep OpenPGP / X.509 / `tag -s` unclaimed; CLI refuses signed-tag / non-SSH `gpg.format` / `verify-tag` fail-closed; **no** invented crypto Claimed/PASS; `spec check` clean.
9. When L8 ships: sequential clean-room Docker product e2e suite green (`g14` + `g16` minimum per `verify/matrices/matrix-l8-e2e.md` / `verify/scripts/run-l8-e2e.sh`; expand as matrix requires); receipts under `verify/gates/*/out/`; **no** host-only Verified; **no** invented PASS after OOM/stampede; `spec check` clean.
10. When L_final ships: public `docs/git-vs-arks.md` matches residual/CLI SoT honesty (Claimed / N/A / reopen-only); nav linked; `spec check` clean.
