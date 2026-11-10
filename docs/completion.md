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

Tip carrying sha1cd detect+refuse + host-env advisory: see [Git vs arks](../git-vs-arks/) (≥ `54c9dc2` / tip ≥ `60f33c2` class). Authoritative residual ledger: [production residual register](https://github.com/modootoday/arks/blob/main/.spec/sot/production-residual-register.sot.md). Canonical Done / Partial / Blocked / Non-goal tip table: [program roadmap](https://github.com/modootoday/arks/blob/main/.spec/sot/program-roadmap.sot.md) (v0.15.2+). Per-gap unblock paths: [partial-completion unblock research](https://github.com/modootoday/arks/blob/main/.spec/research/20261111001816-partial-completion-unblock-roadmap.research.md).

## Documented non-goals

- Absolute SHA-1 collision **immunity** (detect+refuse is Claimed)
- Forge Checks / merge-queue / Issues UX (use `gh`)
- Embedded forge OAuth browser UX
- Full multi-device cloud-sync safety on `.git` (`arks host-env` is **advisory** only)
- Live G8 / scale latency PASS without secrets (honest SKIP)
- crates.io / docs.rs publish timing (**L9** owner policy)

## Tip remasure notes (PF5)

On tip ≥`54c9dc2`, G19 M17 ≤0.8× stays **19/19**. Aggressive ≤0.5× is **Claimed(partial)**: alias PATH process-tax and revert short-wall stay honest `aggressive_na` — **not** invented ≤0.5 PASS; **no** remasure thrash after a real gate FAIL (exit 1 ≠ OOM). Historical **Claimed 8/8** remasure at `79fc9bf` remains archival. Details: [Performance](../performance/) · evidence pack [20261110224814](https://github.com/modootoday/arks/tree/main/.spec/evidence/20261110224814-full-cleanroom-matrix).

## Remaining partials (ranked unblock)

1. Live G8 latency — Environment `g8-live` secrets + receipt (SKIP ≠ PASS)
2. Pf5 ≤0.5× — hold `aggressive_na` unless a single justified amortize wave
3. Host-env advisory deepen (WIN-003 / FS-007 / FS-014) — still not full-sync Claimed
4. **L9** — owner SoT only
5. Keep forge UX / crypto immunity closed as Non-goals

## How maintainers verify

Clean-room Docker only — L8 (`g14`→`g16`→`g20`→`g21`) and S6 (`g38`→`g39`→`g40`) plus the compose matrix. Host CI is advisory. Receipts live under `.spec/evidence/`. Never invent PASS for secrets-absent live gates.

## Next

[Git vs arks](../git-vs-arks/) [Limits](../limits/) [Performance](../performance/) [Contribute](../contribute/)
