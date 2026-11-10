---
id: sot:root/pf5-journey-seed-amortization
title: Pf5 journey seed amortization (≤0.5× reopen)
status: active
created: 2026-11-10T22:00:00Z
updated: 2026-11-10T01:46:00Z
tags: [g19, pf5, perf, amortization, journey, verification]
verify: manual
rel:
  depends_on:
    - sot:root/clean-room-journey-compare
    - sot:root/interop-test-matrix
    - sot:root/production-residual-register
    - sot:root/product-verification-clean-room
    - adr:root/20261110162000
    - plan:root/20261110150000
    - evidence:root/20261110153000
  evidence: [evidence:root/20261110224500, evidence:root/20261110234600, evidence:root/20261111001400, evidence:root/20261110003836, evidence:root/20261110014600]
---

# Pf5 journey seed amortization (≤0.5× reopen)

Version **0.1.4**. Normative harness rules that reopen [[adr:root/20261110162000]] for the eight Pf5 `aggressive_na` journeys without inventing Git semantics. Claim amortization receipt [[evidence:root/20261111001400]] — **Claimed(partial 7/8)** (prior [[evidence:root/20261110234600]] 6/8; [[evidence:root/20261110224500]] 4/8). Latest sole Docker Verified pin [[evidence:root/20261110014600]] (tip `e466d56`; 18/19 ≤0.5×; CLONE-LOCAL remains `aggressive_na` @ 0.6705).

## Rule

1. **Reopen authority.** Pf5 leftovers MAY leave `aggressive_na` only after Docker G19 remeasure under this SoT + [[sot:root/clean-room-journey-compare]]. Host timings remain advisory.
2. **Seed amortization (allowed).** For quantum / process-tax / mid residuals, the G16 harness MAY:
   - enlarge **untimed** identical seeds (same files/argv for git and arks lanes);
   - add **timed** steps that remain catalog-faithful porcelain (`add`/`commit`/`switch`/`bundle`/`stash`/`mv`/`rm`/`clone`/`worktree` …) where arks already competes, so process-spawn floor does not dominate the journey median;
   - share a prepared peer seed across iterations when isolation is preserved (clone bare / HTTP peer already does this).
3. **Process-tax reduction (allowed).** Prefer fewer process boundaries only when the catalog argv sketch still runs as distinct Git-compat commands (no invented multi-command daemon; no semantic skip of locks/fsync). Larger per-commit / per-tree payloads that make real object/index work dominate spawn tax are the default lever.
4. **Forbidden.** Silent keep-at-0.8 as a ≤0.5× substitute; fake greens; CELIR/`gix` shortcuts; changing Git object/ref/index bytes for “wins.”
5. **Claim rule.** On G19, a former Pf5 NA journey is **claimed ≤0.5×** iff measured `arks_median ≤ 0.5 × git_median`. Otherwise it remains `aggressive_na` with this SoT cite. Honest partial umbrella **Claimed(partial N/M)** is allowed.
6. **M17 regression unchanged.** Full catalog stays ≤0.8× N/A-free measured; this SoT does not waive that bar.
7. **Short-wall flicker.** Journeys whose stock-Git median is already low tens of ms (notably **J-CHERRY-01**) MAY land in `aggressive_na` (quantum-timer-floor) when a sole G19 remeasure is `>0.5×` even if an earlier Pf4 receipt once claimed ≤0.5×. Do **not** thrash sole-runner G19 hoping for a lucky ≤0.5× sample.

## Journey class map (frozen inventory)

| ID | Class | Amortization intent |
|----|-------|---------------------|
| J-BRANCH-01 | quantum | Heavier divergent trees + timed commit amortization after switches |
| J-BUNDLE-01 | quantum | Larger blob seed for pack encode + timed commit amortization |
| J-RM-MV-01 | quantum | Larger multi-path rm set + timed commit amortization |
| J-STASH-01 | quantum | Larger dirty tracked set + timed commit amortization |
| J-CHERRY-01 | quantum (flicker) | Keep catalog cherry-pick; classify `aggressive_na` when measured >0.5× on short walls |
| J-LOCAL-01 | process-tax | Larger timed add/commit payloads (arks commit ≪ git) |
| J-ALIAS-01 | process-tax | Same as J-LOCAL-01 under alias shim |
| J-CLONE-LOCAL-01 | mid | Larger shared bare HTTP peer seed (identical for both peers); **exhausted** — further inflation worsened measured ratio; residual NA until product smart-HTTP work |
| J-WORKTREE-01 | mid | Modest tree (avoid checkout regression) + timed commit amortization |

## Verification

1. `spec check` clean for this SoT and cross-links.
2. Sole Docker `g19` receipt; `g19_eval.py` records claimed vs remaining `aggressive_na`.
3. Residual register **FW-PF5-NA** + per-journey P6 rows updated to Claimed / Claimed(partial) / remaining NA.
4. Evidence filed under `.spec/evidence/`.

## Non-goals

- In-process multi-command daemon (future ADR only).
- Redefining M17 ≤0.8× or prior Pf4 claimed ≤0.5× set.
- Public “faster than Git” marketing beyond receipt honesty ([[sot:root/public-surface]]).
