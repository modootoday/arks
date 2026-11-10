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
| Interactive TUI (`rebase -i`, `add -p`, editor-required commit) | Yes | Out of non-interactive agent bar | **N/A** |
| GUI (`gitk`, `git-gui`) / Git LFS / `scalar` | Available as separate tools | Out of product | **N/A** |

## Remotes and server

| Area | Git | arks | Status |
|------|-----|------|--------|
| Smart HTTP clone/fetch/push | Yes | Yes | **Claimed** |
| SSH clone/fetch/push (OpenSSH helper) | Yes | Yes | **Claimed** |
| Client `git://` | Yes | Yes | **Claimed** (host wire; ladder L2) |
| Dumb HTTP push (WebDAV write subset) | Yes (peer-dependent) | Yes (LOCK/MKCOL/PUT/MOVE/DELETE; `:<ref>` delete) | **Claimed** (host wire; L3 + S3) |
| Server smart HTTP / dumb / SSH stdio / `git://` daemon | Yes | Claimed server subset | **Claimed** (M11 / M25 G29–G32) |
| `--depth` shallow clone/fetch | Yes | Yes on pack transports | **Claimed** (G35) |
| Partial-clone `--filter=blob:none` / promisor | Yes | Yes (`blob:none`; other specs refuse) | **Claimed** (R3 / S1 host wire) |
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
| R3 filter / promisor | **Claimed `blob:none` (S1 host wire)**; other specs N/A | Symmetry e2e twin (**S6**); expand filter-specs via SoT |
| R4 OpenPGP / X.509 / `tag -s` | **Claimed (S2 / G38)** | Re-verify G38 after crypto wire change |
| R5 live forge scale PASS | Harness Claimed; ops dry-run Done | Secrets + receipt; never invent latency |
| R6 clean-room Git pin (**2.39.5**) | Process-closed | Deliberate pin bump + G14-class re-prove |
| crates.io / docs.rs timing | Closed owner policy (L9) | Owner decision |

## Product-completion DoD (ladder)

### Historical (L0–L_final) — Done

1. **L8** — **Done**: clean-room Docker suite `g14`→`g16`→`g20`→`g21` PASS (see [matrix-l8-e2e](https://github.com/modootoday/arks/blob/main/verify/matrices/matrix-l8-e2e.md)).
2. **L_final** — **Done**: this comparison matrix ships on Pages; keep honest against Specfold residuals (never invent Claimed).
3. **L9** — Remains closed owner policy unless the owner opens it.

### Expanded symmetry DoD (S0–S_final) — active

| Goal | Focus | Status |
|------|-------|--------|
| **S0** | Inventory lock (implement / SoT-amend / still-OOS) | **Done** |
| **S1** | R3 filter / promisor (`blob:none`) | **Done** (host wire) |
| **S2** | R4 OpenPGP / X.509 / `tag -s` | **Done** (G38) |
| **S3** | Protocol narrows (dumb HTTP deletes) | **Done** (host wire) |
| **S4** | Interactive / patch flows or honest defer | Open |
| **S5** | R5 live forge (secrets) or documented block | Open |
| **S6** | Symmetry e2e matrix (git vs arks) | Open |
| **S7** | Refresh this table + residual register | Open |
| **S_final** | Expanded DoD met; **L9** still closed | Open |

See: [symmetry ladder S0–S_final](https://github.com/modootoday/arks/blob/main/.spec/research/20261110105436-git-symmetry-expanded-dod-goal-ladder-s0-s-final.research.md) · historical [L-ladder](https://github.com/modootoday/arks/blob/main/.spec/research/20261110095142-product-completion-goal-ladder.research.md). Limits: [Limits](../limits/).

## Next

[Limits](../limits/) [Performance](../performance/) [Alias `git` → `arks`](../alias-git-arks/) [Contribute](../contribute/)
