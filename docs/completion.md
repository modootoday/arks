---
title: Completeness
description: Honest YES/PARTIAL bar for git feature symmetry vs documented non-goals
nav_order: 66
schema: {faq: auto}
sections:
  - verdict
  - non-goals
  - verify
  - {c: cta, name: next}
---

# Completeness

Short public answer to “is arks **fully complete**?” — without inventing Claimed or PASS.

## Verdict

| Scope | Answer |
|-------|--------|
| Git feature symmetry DoD (Full Symmetry **FS_final**) | **YES** — Claimed rows carry receipts; residuals are Impossible / advisory / owner-policy, not casual N/A |
| Live forge latency SLOs, crates.io timing (**L9**), absolute crypto immunity, forge host UX, full cloud-sync safety | **NO** — documented non-goals |
| One-word operator summary | **PARTIAL** — symmetry DoD complete; non-FS residuals remain |

Tip carrying Pf5 **19/19 ≤0.5× Claimed** (ALIAS hardlink cut), sha1cd detect+refuse, host-env advisory, forge thin-wrap, server `bundle-uri` + LFS locks + `sparse:oid` slices: see [Git vs arks](../git-vs-arks/) (G-ladder [[plan:root/20261111024440]]; ALIAS Claimed [[evidence:root/20261111032000]]). Authoritative residual ledger: [production residual register](https://github.com/modootoday/arks/blob/main/.spec/sot/production-residual-register.sot.md). Canonical tip table: [program roadmap](https://github.com/modootoday/arks/blob/main/.spec/sot/program-roadmap.sot.md) (v0.15.5+ G-ladder). Gap classification: [20261111022345](https://github.com/modootoday/arks/blob/main/.spec/research/20261111022345-gap-classification-self-dep-ffi.research.md).

## Documented non-goals / still blocked

- Absolute SHA-1 collision **immunity** (detect+refuse is Claimed)
- Full forge Checks / merge-queue / Issues **product** UX (Claimed-closest: `arks forge` → host `gh`)
- Embedded forge OAuth browser UX (external credential.helper honored)
- Full multi-device cloud-sync safety on `.git` (`arks host-env` detect + opt-in refuse only)
- Live G8 / scale latency PASS without secrets (honest SKIP; local-forge ≠ live PASS)
- crates.io / docs.rs publish timing (**L9** owner policy — reopen pack only)

## Tip remasure notes (PF5)

G19 M17 ≤0.8× stays **19/19**. Aggressive ≤0.5×: hardlink + no PATH rewrite → **exactly one** sole Docker G19 → **claimed=19** / `aggressive_na=0` (ALIAS **0.4926**, REVERT **0.4227**) ([[evidence:root/20261111032000]]); prior G7 `aggressive_na` **0.5489** archival ([[evidence:root/20261111025200]]); **no** stampede; halt OOM. Historical **Claimed 8/8** at `79fc9bf` remains archival. Details: [Performance](../performance/).

## Remaining partials (ranked unblock)

1. Live G8 latency — Environment `g8-live` secrets still **0** ([[evidence:root/20261111032001]]) + receipt (SKIP ≠ PASS) — **Operator-only**
2. Host-env — Claimed detect+opt-in refuse / LSM detect; full sync still Non-goal / Impossible
3. **L9** — owner SoT reopen pack only (G2 refreshed) — **Operator-only**
4. Crypto immunity / full sync / forge product UX / CELIR ObjectStore / multi-writer warranty — Hold forever (detect / thin-wrap Claimed; **never** fake)

Detail: [G-ladder plan](https://github.com/modootoday/arks/blob/main/.spec/plan/20261111024440-gap-classification-g-ladder.plan.md) · [20261111022345](https://github.com/modootoday/arks/blob/main/.spec/research/20261111022345-gap-classification-self-dep-ffi.research.md).

## How maintainers verify

Clean-room Docker only — L8 (`g14`→`g16`→`g20`→`g21`) and S6 (`g38`→`g39`→`g40`) plus the compose matrix. Host CI is advisory. Receipts live under `.spec/evidence/`. Never invent PASS for secrets-absent live gates.

## Next

[Git vs arks](../git-vs-arks/) [Limits](../limits/) [Performance](../performance/) [Contribute](../contribute/)
