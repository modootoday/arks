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

Tip carrying Pf5 REVERT ≤0.5× Claimed + ALIAS PATH/`exec` G7 remasure, sha1cd detect+refuse, host-env advisory, forge thin-wrap, server `bundle-uri` + LFS locks slices: see [Git vs arks](../git-vs-arks/) (G-ladder [[plan:root/20261111024440]]; prior unblock [[evidence:root/20261111015154]]). Authoritative residual ledger: [production residual register](https://github.com/modootoday/arks/blob/main/.spec/sot/production-residual-register.sot.md). Canonical tip table: [program roadmap](https://github.com/modootoday/arks/blob/main/.spec/sot/program-roadmap.sot.md) (v0.15.5+ G-ladder). Gap classification: [20261111022345](https://github.com/modootoday/arks/blob/main/.spec/research/20261111022345-gap-classification-self-dep-ffi.research.md).

## Documented non-goals / still blocked

- Absolute SHA-1 collision **immunity** (detect+refuse is Claimed)
- Full forge Checks / merge-queue / Issues **product** UX (Claimed-closest: `arks forge` → host `gh`)
- Embedded forge OAuth browser UX (external credential.helper honored)
- Full multi-device cloud-sync safety on `.git` (`arks host-env` detect + opt-in refuse only)
- Live G8 / scale latency PASS without secrets (honest SKIP; local-forge ≠ live PASS)
- crates.io / docs.rs publish timing (**L9** owner policy — reopen pack only)

## Tip remasure notes (PF5)

G19 M17 ≤0.8× stays **19/19**. Aggressive ≤0.5×: G7 PATH/`exec` absolute-shim cut → **exactly one** sole Docker G19 → ALIAS `aggressive_na` **0.5489**, REVERT **0.4943 Claimed** ([[evidence:root/20261111025200]]); **no** stampede; halt OOM. Historical **Claimed 8/8** at `79fc9bf` remains archival. Details: [Performance](../performance/).

## Remaining partials (ranked unblock)

1. Live G8 latency — Environment `g8-live` secrets still **0** (2026-11-11 G1 re-probe) + receipt (SKIP ≠ PASS) — **Operator-only**
2. Pf5 ALIAS ≤0.5× — G7 PATH/`exec` cut landed; disposition follows sole G19 receipt — **no thrash**
3. Host-env — Claimed detect+opt-in refuse / LSM detect; full sync still Non-goal / Impossible
4. **L9** — owner SoT reopen pack only (G2 refreshed) — **Operator-only**
5. Crypto immunity / full sync / forge product UX / CELIR ObjectStore / multi-writer warranty — Hold forever (detect / thin-wrap Claimed; **never** fake)

Detail: [G-ladder plan](https://github.com/modootoday/arks/blob/main/.spec/plan/20261111024440-gap-classification-g-ladder.plan.md) · [20261111022345](https://github.com/modootoday/arks/blob/main/.spec/research/20261111022345-gap-classification-self-dep-ffi.research.md).

## How maintainers verify

Clean-room Docker only — L8 (`g14`→`g16`→`g20`→`g21`) and S6 (`g38`→`g39`→`g40`) plus the compose matrix. Host CI is advisory. Receipts live under `.spec/evidence/`. Never invent PASS for secrets-absent live gates.

## Next

[Git vs arks](../git-vs-arks/) [Limits](../limits/) [Performance](../performance/) [Contribute](../contribute/)
