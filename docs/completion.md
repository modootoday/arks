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

Tip carrying sha1cd detect+refuse + host-env deepen + forge thin-wrap: see [Git vs arks](../git-vs-arks/) (tip ≥ `307d509` / U-ladder [[evidence:root/20261111003444]]). Authoritative residual ledger: [production residual register](https://github.com/modootoday/arks/blob/main/.spec/sot/production-residual-register.sot.md). Canonical Done / Partial / Blocked / Non-goal tip table: [program roadmap](https://github.com/modootoday/arks/blob/main/.spec/sot/program-roadmap.sot.md) (v0.15.3+). Per-gap unblock paths: [partial-completion unblock research](https://github.com/modootoday/arks/blob/main/.spec/research/20261111001816-partial-completion-unblock-roadmap.research.md).

## Documented non-goals / still blocked

- Absolute SHA-1 collision **immunity** (detect+refuse is Claimed)
- Full forge Checks / merge-queue / Issues **product** UX (Claimed-closest: `arks forge` → host `gh`)
- Embedded forge OAuth browser UX (external credential.helper honored)
- Full multi-device cloud-sync safety on `.git` (`arks host-env` detect + opt-in refuse only)
- Live G8 / scale latency PASS without secrets (honest SKIP; local-forge ≠ live PASS)
- crates.io / docs.rs publish timing (**L9** owner policy — reopen pack only)

## Tip remasure notes (PF5)

G19 M17 ≤0.8× stays **19/19**. Aggressive ≤0.5×: U1 applied one controlled amortize wave for ALIAS/REVERT — sole remasure only; **no** stampede after FAIL. If still miss: keep **Claimed(partial)** with honest `aggressive_na`. Historical **Claimed 8/8** at `79fc9bf` remains archival. Details: [Performance](../performance/) · [20261110224814](https://github.com/modootoday/arks/tree/main/.spec/evidence/20261110224814-full-cleanroom-matrix) · [20261111003444](https://github.com/modootoday/arks/tree/main/.spec/evidence/20261111003444-u-ladder-partial-gap-close).

## Remaining partials (ranked unblock)

1. Live G8 latency — Environment `g8-live` secrets + receipt (SKIP ≠ PASS)
2. Pf5 ≤0.5× — U1 wave done; hold unless product-justified further change (no thrash)
3. Host-env — Claimed detect+opt-in refuse / LSM detect; full sync still Non-goal
4. **L9** — owner SoT reopen pack only
5. Crypto immunity — Impossible (docs only)

## How maintainers verify

Clean-room Docker only — L8 (`g14`→`g16`→`g20`→`g21`) and S6 (`g38`→`g39`→`g40`) plus the compose matrix. Host CI is advisory. Receipts live under `.spec/evidence/`. Never invent PASS for secrets-absent live gates.

## Next

[Git vs arks](../git-vs-arks/) [Limits](../limits/) [Performance](../performance/) [Contribute](../contribute/)
