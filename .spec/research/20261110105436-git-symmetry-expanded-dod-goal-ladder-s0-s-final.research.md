---
id: research:root/20261110105436
title: Git symmetry expanded DoD goal ladder (S0-S_final)
status: active
created: 2026-11-10T10:54:36Z
updated: 2026-11-10T10:58:55Z
tags:
  [
    git-symmetry,
    expanded-dod,
    residual,
    r3,
    r4,
    r5,
    s0-s-final,
    clean-room-e2e,
    comparison-matrix,
  ]
rel:
  depends_on:
    - sot:root/production-residual-register
    - sot:root/arks-remote
    - sot:root/arks-object
    - sot:root/cli-surface
    - sot:root/public-surface
    - research:root/20261110095142
    - sot:root/arks-server
    - sot:root/product-intent
    - sot:root/interop-test-matrix
    - sot:root/clean-room-journey-compare
---

# Git symmetry expanded DoD goal ladder (S0–S_final)

## Question

After ladder **L0–L8 + L_final Done** ([[research:root/20261110095142]]) with **R3/R4** closed as honest N/A (refuse ≠ Claimed) and **L9** owner-closed, what ordered **git↔arks symmetry** goals reopen implementable residuals so the expanded product DoD is met with Specfold honesty (SoT amend before Claimed; no fake PASS; no OWNER.md; no force-push)?

## Method

1. Inventory reopen-only **R3–R6**, §F **N/A** rows, and comparison-table gaps in `docs/git-vs-arks.md` against [[sot:root/production-residual-register]] / owning SoTs.
2. Classify each residual as **implement** / **SoT-amend-then-implement** / **secrets-blocked** / **still-out-of-scope** / **owner-closed**.
3. Order goals by Specfold honesty first (SoT amend before wire) → highest-value protocol/CLI reopen → symmetry e2e → docs/table currency.
4. Keep **L9** closed unless an explicit in-repo owner reopen appears. Keep true product OOS (GUI / LFS end-product / scalar / forge Issues·PR·Checks UX / agent·IDE runtimes) out unless a primary SoT already claims them.

## Findings

### Honesty baseline (tip ≥ `7500f24`)

| Item | State |
|------|-------|
| Prior ladder | **L0–L8 + L_final Done**; **L9** closed owner policy |
| R3 / R4 | Honest **N/A** (CLI refuse); reopen = SoT amend + wire + gate |
| R5 | Harness Claimed + L5 ops dry-run; **≠** live latency PASS |
| R6 | Process-closed pin **2.39.5** (not this DoD) |
| §F | **38/38** terminal; reopen only via ADR/SoT + re-verify |
| Comparison | `docs/git-vs-arks.md` current for L-era honesty |

### S0 — Inventory lock (this note)

| Residual / gap | Prior disposition | Expanded DoD class | Ladder goal |
|----------------|-------------------|--------------------|-------------|
| **R3** filter / promisor (`blob:none` claimed; other specs refuse) | N/A (L6 refuse) → **Claimed host wire** | **SoT-amend → implement** | **S1 Done** ([[evidence:root/20261110105855]]) |
| **R4** OpenPGP / X.509 / `tag -s` / `verify-tag` | N/A (L7 refuse) | **SoT-amend → implement** | **S2** |
| Dumb HTTP push **deletes** (`-d`/`-D`) | Explicit non-claim in [[adr:root/20261110190000]] | **SoT-amend → implement** if WebDAV DELETE SoTable | **S3** |
| Other §F protocol narrows still refuse-only (if any SoTable) | Claimed subset / N/A | Inventory then amend | **S3** |
| Interactive TUI (`rebase -i`, `add -p`, editor-required commit) | N/A (agent non-interactive bar) | **Implement if Specfold allows**; else honest defer with SoT reason | **S4** |
| **R5** live forge scale / G8 live PASS | Ops readiness Done | **Live only if secrets**; else structural + documented block (**≠** fake PASS) | **S5** |
| Symmetry e2e matrix (git vs arks clean-room) | L8 = g14/g16/g20/g21 | **Expand** for each newly Claimed surface | **S6** |
| `docs/git-vs-arks.md` + residual register | L_final Done | Refresh Claimed/PASS with evidence only | **S7** |
| **R6** Git pin advance | Process-closed | **Still process-closed** (not expanded DoD) | — |
| **L9** crates.io / docs.rs timing | Owner-closed | **Owner-closed** (do not reopen) | — |
| **FW-GUI** / **FW-SCALAR** / **FW-LFS** / **FW-FORGE-B** | N/A exclusion | **Still out of scope** | — |
| §F P5 catalog N/A×9 (FS-007, FS-014, HOOK-008, MAINT-004, MISC-006, NET-006, OBJ-004, REF-017, WIN-003) | N/A | **Still out** (hostile / host-OS / forge-shadow / concurrency edge) unless a primary SoT later claims a **narrow protocol** slice | — |
| Forge OAuth under CRED | Refuse | **Still Refuse** (policy) | — |
| Credential helper / askpass / TTY / OS wallets | G33 Claimed | Already Claimed; no reopen | — |

### Ordered symmetry ladder

| # | Goal | Class | Exit criteria | Status |
|---|------|-------|---------------|--------|
| **S0** | Inventory lock | Specfold | This table maps every residual → implement / SoT-amend / still-OOS / owner-closed; residual register + AGENTS + comparison pointer cite this research | **Done** (this note) |
| **S1** | **R3** partial-clone filter / promisor | Git asymmetry | SoT amend [[sot:root/arks-remote]] beyond `--depth`; wire `filter` + `blob:none`; promisor / `extensions.partialClone`; CLI support claimed spec; host evidence before Claimed | **Done** ([[adr:root/20261110105547]] + [[evidence:root/20261110105855]]) |
| **S2** | **R4** OpenPGP / X.509 / `tag -s` / `verify-tag` | Git asymmetry | SoT amend [[sot:root/arks-object]] + CLI; dedicated crypto harness; G37-class re-verify; evidence before Claimed | **Next** |
| **S3** | Protocol narrows (dumb HTTP deletes / other implementable §F rows) | Git asymmetry | Per-row ADR + SoT amend; host/clean-room evidence; no invent Claimed | Open |
| **S4** | Interactive / patch flows | CLI | Specfold allows → implement + e2e; else SoT reason + honest defer (**≠** fake PASS) | Open |
| **S5** | **R5** live forge scale | Secrets-gated | Secrets present → measured receipt; else documented structural block; never invent latency PASS | Open |
| **S6** | Symmetry e2e matrix | Verify DoD | Clean-room Docker git vs arks for each newly Claimed surface; expand beyond L8; suite green; halt on OOM/stampede | Open |
| **S7** | Comparison + residual currency | Docs / DoD | `docs/git-vs-arks.md` + [[sot:root/production-residual-register]] match new Claimed/PASS with evidence ids | Open |
| **S_final** | Expanded DoD met | Product | Implementable residuals closed with evidence; symmetry suite green; comparison table current; **L9** still closed | Open |

### Non-goals

- Treating **L9** as this DoD.
- Inventing Claimed/PASS without SoT amend + carrying evidence.
- Restarting sole G19 remasure without honesty breakage.
- Force-push / OWNER.md / fake live forge PASS.
- Relitigating true product OOS (GUI, LFS end-product, scalar, forge host UX, agent/IDE runtimes).

### Execution note (this round)

**S0–S1 Done** on tip: inventory lock + R3 `blob:none` host-wire Claimed. Next **S2** (R4 OpenPGP / X.509 / `tag -s`). Symmetry clean-room expansion remains **S6**. Prior L-ladder remains historical; this **S-*** ladder is the active expanded DoD order.

## Verification

1. Residual register / AGENTS / `docs/git-vs-arks.md` cite this research as the active symmetry ladder (L-ladder remains historical Done).
2. S0 table classifies every R3–R6 + relevant §F N/A + comparison gaps without inventing Opens as bare development items.
3. When S1 ships: [[sot:root/arks-remote]] claims filter/promisor subset with primary-doc cites; residual **R3** flips from N/A; CLI supports claimed filter-specs; evidence + matrix PASS; `spec check` clean.
4. When S2 ships: [[sot:root/arks-object]] + CLI claim crypto subset with harness evidence; **R4** flips; `spec check` clean.
5. When S3–S4 ship: each row has SoT/ADR disposition (Claimed or honest defer); no silent Claimed.
6. When S5 ships: either live receipt or explicit secrets-block doc; never invented latency.
7. When S6 ships: sequential clean-room symmetry suite green; receipts under `verify/`; no host-only Verified substitute.
8. When S7 / S_final ship: comparison table + residual register match evidence; **L9** still closed; `spec check` clean.
