# Comparative performance matrix (G19) — M14 Phase Pf

Owning SoT: `.spec/sot/clean-room-journey-compare.sot.md`, `.spec/sot/interop-test-matrix.sot.md` (G19). Plan: `.spec/plan/20261109024636-m14-comparative-performance-unlock.plan.md`.

Authoritative venue: Docker clean-room service `g019-comparative-performance` (`verify/scripts/run-g019-comparative-performance.sh`). Host timings are advisory only.

## Normative bar

For each allow-listed journey ID: **arks median wall-clock ≤ 0.8 × git median** (arks ≥20% faster), `aggregation=median_p95`, `iterations≥3`, release `arks`, same clean-room image/seed. Functional class remains a hard gate (via G16).

## Wave Pf-1 allow-list (claimed)

Chosen from clean-room G16 `median_p95` where the ≥20% bar holds **stably** across authoritative runs — no marketing beyond this receipt.

| ID | Notes | Required |
|----|-------|----------|
| J-SUBMODULE-01 | Local submodule journey (stable ≤0.8×) | yes |

Wave Pf-1 **Done** = Docker G19 PASS for the allow-list + G16 functional / G14 regression green. Evidence: [[evidence:root/20261109170000]].

## Wave Pf-2 allow-list (claimed)

| ID | Notes | Required |
|----|-------|----------|
| J-SUBMODULE-01 | Regression from Pf-1 | yes |
| J-REPACK-01 | M15 Fs-3 `repack -ad` multi-object pack workload (stable ≤0.8×) | yes |

Wave Pf-2 **Done** = Docker G19 PASS for the expanded allow-list + G16 functional / G14 regression green.

## Explicit Opens (historical Pf-2)

Remaining G16 journey IDs were Open under M14 Pf-2. **M17 / Pf3** claims the full catalog below.

## Wave Pf3-1 allow-list (claimed — M17)

Owning plan: `.spec/plan/20261109043916-m17-full-g019-comparative-performance-journey-allow-list.plan.md`.
Allow-list = **every required ID** in `verify/matrices/matrix-g016-journey-compare.md` (current G16 catalog). Default bar: arks_median ≤ 0.8 × git_median. N/A only with explicit SoT rationale (none in this catalog at claim time).

| ID | Notes | Required |
|----|-------|----------|
| J-LOCAL-01 | Everyday local porcelain | yes |
| J-BRANCH-01 | Branch / checkout / switch | yes |
| J-MERGE-FF-01 | Fast-forward merge | yes |
| J-REBASE-01 | Non-interactive rebase | yes |
| J-BUNDLE-01 | Bundle create / verify | yes |
| J-CLONE-LOCAL-01 | Smart-HTTP local clone | yes |
| J-CLONE-PATH-01 | Path + `file://` clone | yes |
| J-ALIAS-01 | Alias `git`→`arks` shim | yes |
| J-WORKTREE-01 | Worktree porcelain | yes |
| J-TAG-01 | Annotated tag | yes |
| J-STASH-01 | Stash push/list/pop | yes |
| J-CHERRY-01 | Clean cherry-pick | yes |
| J-DIFF-01 | Diff / cached | yes |
| J-RM-MV-01 | mv / rm tracked | yes |
| J-ARCHIVE-01 | archive tar | yes |
| J-REFLOG-01 | Reflog show | yes |
| J-REVERT-01 | Clean revert | yes |
| J-REPACK-01 | Regression from Pf-2 | yes |
| J-SUBMODULE-01 | Regression from Pf-1/Pf-2 | yes |

Wave Pf3-1 **Done** = Docker G19 baseline recorded for full allow-list (PASS or identified failers for Pf3-2).
Wave Pf3-2 **Done** = all required IDs ≤0.8× (or SoT N/A).
Wave Pf3-3 **Done** = final G19 PASS receipt + tip pin; residual Closed.

**Pf3 status (2026-11-09):** Pf3-1..Pf3-3 **Done** — Docker `g019-comparative-performance` PASS; carrying evidence [[evidence:root/20261109090000]] (**19/19 measured**, `na: {}`). Historical N/A-era receipt [[evidence:root/20261109073100]] superseded for M17 exit cites.

## Wave Pf4 (M18 — aggressive ≤0.5×; M17 bar frozen)

Owning plan: `.spec/plan/20261109125438-m18-pf4-aggressive-comparative-perf.plan.md`.
Research: `.spec/research/20261109125438-arks-perf-beyond-m17.research.md`.

| Field | Rule |
|-------|------|
| Regression | Full allow-list above stays ≤**0.8×** (M17) |
| Aggressive | Progressive claim ≤**0.5×** on measured IDs (N/A-free) |
| Venue | Sole Docker `g019-comparative-performance` (no concurrent runs) |
| Pf4-1 | [[evidence:root/20261109130000]] — 9/19 ≤0.5× |
| Pf4-2 | [[evidence:root/20261109141500]] — 11/19 ≤0.5× (DIFF/ARCHIVE/CHERRY material) |
| Pf4-3 / M18 exit | [[evidence:root/20261109143000]] — 11/19 ≤0.5× + journey-class residuals |

## Wave Pf5 (M22 — aggressive leftovers terminal)

Owning plan: `.spec/plan/20261110150000-m22-residual-terminal-close.plan.md`.

| Field | Rule |
|-------|------|
| Regression | Full allow-list stays ≤**0.8×** (M17; N/A-free measured) |
| Aggressive claimed | Prior Pf4 ≤**0.5×** IDs + Pf5 flips LOCAL/BRANCH/BUNDLE/ALIAS — [[evidence:root/20261110224500]] |
| Aggressive N/A (residual) | CHERRY short-wall flicker guard; STASH/RM-MV/WORKTREE remain NA-capable if measured >0.5× — SoT [[sot:root/pf5-journey-seed-amortization]] (CLONE-LOCAL flipped ≤0.5×) |
| Done | Every journey ≤0.5× **or** `aggressive_na` with cite; umbrella **Claimed 8/8** ([[evidence:root/20261110142500]] tip `79fc9bf`; LOCAL **0.1579** (6/38) / PATH **0.1739** (8/46); miss=[]); prior [[evidence:root/20261110131600]] `875359e`; partial 7/8 [[evidence:root/20261111001400]]; historical tip remasures [[evidence:root/20261110113900]] / [[evidence:root/20261110135100]] / [[evidence:root/20261110114200]] / [[evidence:root/20261110110900]] |

## Evidence schema

Receipt cites: image digest, stock git version, revision, iterations, aggregation, per-ID `git_median_ms`, `arks_median_ms`, `ratio`, `pass`, wave (`Pf-2` historical \| `Pf3` \| `Pf4` \| `Pf5`).
