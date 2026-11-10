---
title: Git vs arks
description: Honest feature comparison — Claimed, N/A, and reopen-only residuals
nav_order: 65
schema: {faq: auto}
sections:
  - how
  - core
  - remotes
  - signing
  - out
  - {c: faq, name: reading}
  - {c: cta, name: next}
---

# Git vs arks

Side-by-side product honesty for stock **Git** vs **arks**. Status labels are Specfold-backed — **Claimed** only with receipts, **N/A** when out of claim, **reopen-only** when a residual may return via SoT amend. This page does **not** invent PASS or Claimed.

> [!NOTE]
> Authoritative ledgers live in the source tree under `.spec/` (especially the [production residual register](https://github.com/modootoday/arks/blob/main/.spec/sot/production-residual-register.sot.md) and [CLI surface](https://github.com/modootoday/arks/blob/main/.spec/sot/cli-surface.sot.md)). Clean-room Docker gates prove Verified claims — host CI alone does not.

## How to read status

| Label | Meaning |
|-------|---------|
| **Claimed** | Product surface with carrying evidence / clean-room receipt |
| **Done (ops)** | Maintainer readiness landed; not a live publish / latency PASS |
| **N/A** | Explicitly out of claim; refuse-closed or excluded |
| **Reopen-only** | Residual may return only via SoT amend + re-verify |
| **Closed (policy)** | Owner / process closed; not a development Open |

## Core CLI and same-`.git`

| Area | Git | arks | Status |
|------|-----|------|--------|
| Native CLI on same `.git` | Yes | Yes (in-engine; no product-path `git` wrapper) | **Claimed** (M9 / G14) |
| Everyday porcelain (`add`/`commit`/`status`/`log`/branch/checkout/…) | Yes | Yes for claimed inventory | **Claimed** (M9–M16) |
| Full CLI inventory + flag symmetry (pin) | Yes | Implement or N/A exclusion (M16 bar) | **Claimed** (G20 / G21) |
| `git` → `arks` alias drop-in | N/A | Documented alias; muscle-memory argv | **Claimed** (M16 / G21) |
| Clean-room journey catalog (19 scenarios) | Oracle peer | Func PASS in Docker G16 | **Claimed** (G16 / G19 venue) |
| Interactive TUI (`rebase -i`, `add -p`, editor-required commit) | Yes | Scripted/non-TTY Claimed (`GIT_SEQUENCE_EDITOR` / `ARKS_EDITOR` / `ARKS_ADD_PATCH_ANSWERS`); full curses TUI N/A | **Claimed** (O3 scripted) / **N/A** (pixel TUI) |
| GUI (`gitk`, `git-gui`) / Git LFS / `scalar` | Available as separate tools | Out of product | **N/A** |

## Remotes and server

| Area | Git | arks | Status |
|------|-----|------|--------|
| Smart HTTP clone/fetch/push | Yes | Yes | **Claimed** |
| SSH clone/fetch/push (OpenSSH helper) | Yes | Yes | **Claimed** |
| Client `git://` | Yes | Yes | **Claimed** (host wire; ladder L2) |
| Dumb HTTP push (WebDAV write subset) | Yes (peer-dependent) | Yes (LOCK/MKCOL/PUT/MOVE/DELETE; `:<ref>` delete); Apache `mod_dav` twin = G41 when receipted | **Claimed** (host wire; L3 + S3); Apache **reopen-only** until G41 |
| `push --force-with-lease` / `--push-option` | Yes | Yes on smart pack (lease + options→hooks); not on dumb | **Claimed** (O4 host wire) |
| Server smart HTTP / dumb / SSH stdio / `git://` daemon | Yes | Claimed server subset | **Claimed** (M11 / M25 G29–G32) |
| `--depth` shallow clone/fetch | Yes | Yes on pack transports | **Claimed** (G35) |
| Partial-clone `--filter=blob:none`/`tree:0` / promisor | Yes | Yes (`blob:none` + `tree:0`; other specs refuse) | **Claimed** (R3 / S1 / O2 host wire) |
| Forge OAuth / Checks / merge-queue UX | Via host tools | Out of product | **N/A** / **Refuse** |

## Signing and credentials

| Area | Git | arks | Status |
|------|-----|------|--------|
| SSH commit sign / verify (`gpg.format=ssh`) | Yes | Yes (`commit -S` / `verify-commit`) | **Claimed** (G37) |
| OpenPGP / X.509 commit signing | Yes | Yes (`gpg.format=openpgp` / `x509`) | **Claimed** (R4 / S2 / G38) |
| Signed tags (`tag -s`) / `verify-tag` | Yes | Yes | **Claimed** (R4 / S2 / G38) |
| Credential helper / askpass / TTY / OS wallets | Yes | Claimed first-party subset | **Claimed** (G33) |

## Performance and concurrency

| Area | Git | arks | Status |
|------|-----|------|--------|
| Clean-room journey ≤0.8× allow-list | Oracle | Measured 19/19 | **Claimed** (M17 / G19) |
| Aggressive ≤0.5× journeys | Oracle | **Claimed 8/8** (incl. Clone HTTP) | **Claimed** (PF5) |
| Product leases / high-concurrency shape | N/A (stock) | Claimed with receipts | **Claimed** (M10 / M23 / M26) |
| Live forge latency SLO | N/A | Harness ready; no invented live PASS | **Done (ops)** (L5) ≠ live PASS |

## Out of claim / reopen-only

| Residual | Disposition | Reopen requires |
|----------|-------------|-----------------|
| R3 filter / promisor | **Claimed `blob:none` + `tree:0` (S1 / O2)**; other specs N/A | Expand further filter-specs via SoT |
| R4 OpenPGP / X.509 / `tag -s` | **Claimed (S2 / G38; S6 re-PASS)** | Re-verify G38 after crypto wire change |
| R5 live forge scale PASS | Harness Claimed; **O1 secrets hard-block** (Environment `g8-live` secret count=0) | Secrets + receipt; never invent latency |
| Apache DAV dumb twin | Structural G41 matrix; Claimed only with receipt | Docker G41 PASS |
| R6 clean-room Git pin (**2.39.5**) | Process-closed | Deliberate pin bump + G14-class re-prove |
| crates.io / docs.rs timing | Closed owner policy (L9) | Owner decision |

## Product-completion DoD (ladder)

### Historical (L0–L_final) — Done

1. **L8** — **Done**: clean-room Docker suite `g14`→`g16`→`g20`→`g21` PASS (see [matrix-l8-e2e](https://github.com/modootoday/arks/blob/main/verify/matrices/matrix-l8-e2e.md)).
2. **L_final** — **Done**: this comparison matrix ships on Pages; keep honest against Specfold residuals (never invent Claimed).
3. **L9** — Remains closed owner policy unless the owner opens it.

### Expanded symmetry DoD (S0–S_final) — Done

| Goal | Focus | Status |
|------|-------|--------|
| **S0–S_final** | See prior tip | **Done** (**L9** closed) |

### Optional residuals (O1–O_final) — active

| Goal | Focus | Status |
|------|-------|--------|
| **O1** | Live G8 remasure | **Done (secrets hard-block)** — Environment secrets=0 |
| **O2** | Extra filter-specs (`tree:0`) | **Done** (host wire) |
| **O3** | Interactive scripted subset | **Done** (scripted Claimed; pixel TUI N/A) |
| **O4** | force-with-lease / push-options / Apache DAV | **Done** (lease+options host); Apache **G41 pending receipt** |
| **O_final** | Docs / residual currency; **L9** closed | **Done** |

See: [O-ladder](https://github.com/modootoday/arks/blob/main/.spec/research/20261110205403-optional-residuals-o-ladder-o1-o-final.research.md) · [symmetry S0–S_final](https://github.com/modootoday/arks/blob/main/.spec/research/20261110105436-git-symmetry-expanded-dod-goal-ladder-s0-s-final.research.md). Limits: [Limits](../limits/).

## Next

[Limits](../limits/) [Performance](../performance/) [Alias `git` → `arks`](../alias-git-arks/) [Contribute](../contribute/)
