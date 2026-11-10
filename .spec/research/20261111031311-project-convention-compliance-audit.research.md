---
id: research:root/20261111031311
title: Project convention compliance audit (tip ≥ 261f0e3)
status: active
created: 2026-11-10T18:13:11Z
updated: 2026-11-10T18:13:11Z
tags:
  [
    audit,
    conventions,
    compliance,
    specfold,
    residual-register,
    evidence,
    public-surface,
    english-only,
    ci-hygiene,
    g-ladder,
    u-ladder,
  ]
verify: manual
rel:
  depends_on:
    - sot:root/program-roadmap
    - sot:root/production-residual-register
    - sot:root/public-surface
    - sot:root/language-english-only
    - sot:root/product-verification-clean-room
    - plan:root/20261111024440
    - plan:root/20261111024441
    - research:root/20261111022345
    - research:root/20261111012840
    - research:root/20261111001816
    - research:root/20261111030858
    - evidence:root/20261111025000
    - evidence:root/20261111025200
    - evidence:root/20261111025300
    - evidence:root/20261111015154
    - evidence:root/20261111003444
---

# Project convention compliance audit (tip ≥ `261f0e3`)

## Question

At tip ≥ **`261f0e3`** (pulled `main` at audit start; may advance), does recent tip work (G-ladder, U-ladder, research notes, public docs, CI) follow the repo’s own Specfold / SoT / public-surface / residual / evidence / English / CI conventions — and which gaps are safe to fix vs recommend-only?

## Method

1. Read router + discipline: `AGENTS.md`, `CONTRIBUTING.md`, `.agents/rules/{00,05,10,20,80}-*.md`, [[sot:root/public-surface]], [[sot:root/production-residual-register]], [[sot:root/program-roadmap]] tip / G-ladder tables.
2. Sample tip commits `7c172f6`…`261f0e3` (G-ladder land + fmt/clippy follow-ups + Blocked/Hold survey) and U-/external-deps wave `6ecd3e0` / `ade4040`.
3. Cross-check Claimed surfaces ↔ evidence packs; residual §F ↔ tip ratios; human `docs/` ↔ Specfold honesty; forbidden patterns (OWNER.md, invent PASS, G19 thrash, force-push folklore).
4. Apply **small, safe doc/SoT drift fixes** in the same change set; leave large refactors as recommendations. Avoid parallel implementer feature crates.

## Audit tip baseline

| Item | Value |
|------|-------|
| Tip at audit start | **`261f0e3`** (*Document why tip Blocked/Hold/NA residuals stay limited.*) |
| Prior feature land | **`7c172f6`** G-ladder G1–G_final |
| CI hygiene follow-ups | **`1590fa2`** rustfmt; **`b7720d9`** clippy |
| Operator honesty | **PARTIAL** (roadmap tip table) |
| G_final claim | Tip + docs + residual + evidence aligned ([[sot:root/program-roadmap]]) |

## Findings

Verdict labels: **PASS** = convention followed; **WARN** = followable gap / process smell; **FAIL** = contradicts owning SoT / hard rule (fixed here when small).

### 1. Specfold-first / SoT before Claimed

| Verdict | Cite | Note |
|---------|------|------|
| **PASS** | [[plan:root/20261111024440]], [[plan:root/20261111024441]], [[research:root/20261111022345]] → `7c172f6` | G-ladder was designed (research → plan → directive) before Claimed land. |
| **PASS** | [[research:root/20261111001816]] → U-ladder `6ecd3e0`; [[research:root/20261111012840]] → `ade4040` | Partial / external-deps waves likewise Specfold-first. |
| **WARN** | [[evidence:root/20261111025300]] | G3–G6 Claimed with **host** unit/CLI evidence. Directive G_final expects clean-room/symmetry e2e for *new* Claimed surfaces; pack itself defers e2e to “G_final / CI”. Host CI ≠ Verified ([[sot:root/product-verification-clean-room]]). Recommend a thin clean-room twin (or explicit SoT “host Claimed / Docker deferred”) before marketing Verified. |

### 2. Residual register currency vs tip

| Verdict | Cite | Note |
|---------|------|------|
| **FAIL** → **fixed** | [[sot:root/production-residual-register]] §F **FW-PF5-NA** / **FW-PF5-J-ALIAS** / **FW-PF5-J-REVERT** | Still cited ALIAS **0.5546** / REVERT **0.5000** from [[evidence:root/20261111015154]] after G7 tip currency moved to **0.5489** / **0.4943** ([[evidence:root/20261111025200]]). Roadmap tip table was current; residual lagged. Updated to v**0.9.10**. |
| **FAIL** → **fixed** | Same SoT Verification §3 | Still required public PF5 honesty as tip **8/8** Claimed ≤0.5×; tip is **Claimed(partial)**. Aligned to tip evidence + archival 8/8. |
| **PASS** | Same SoT **FW-LFS** / **FW-CAT-NET-006** | G-ladder commit did refresh locks / `bundle-uri` rows with [[evidence:root/20261111025300]]. |
| **PASS** | [[research:root/20261111030858]] | Blocked/Hold survey at tip ≥ `b7720d9` is research-only; no invent PASS / no G19. |

### 3. Evidence packs for Claimed surfaces

| Verdict | Cite | Note |
|---------|------|------|
| **PASS** | [[evidence:root/20261111025000]] | G1/G2 Blocked honesty (secrets=0; L9 pack only). |
| **PASS** | [[evidence:root/20261111025200]] | Sole Docker G19 receipt + metrics; exactly one remasure; ALIAS kept `aggressive_na`. |
| **PASS** | [[evidence:root/20261111025300]] | G3–G6 host index with non-claims retained (tus/S3, `sparse:oid`, forge UX, CDN). |
| **PASS** | [[evidence:root/20261111003444]], [[evidence:root/20261111015154]] | U-ladder / REVERT-first packs present with honest Blocked / Claimed-or-na. |
| **WARN** | [[evidence:root/20261111025300]] `verify: host` | Acceptable for Claimed(host) if public copy does not say Verified; watch Pages wording. |

### 4. Public docs drift (`docs/` vs Specfold)

| Verdict | Cite | Note |
|---------|------|------|
| **PASS** | `docs/git-vs-arks.md`, `docs/completion.md` | Tip ALIAS **0.5489** / G-ladder G3–G6 honesty largely current. |
| **FAIL** → **fixed** | `docs/limits.md` Partial-clone paragraph | Still said “other filter-specs remain refused” after G6 Claimed `object:type` / `combine:`. Aligned to tip + `sparse:oid` refuse. |
| **FAIL** → **fixed** | `docs/contribute.md` maintainer filter blurb | Same exotic-filter lag; also LFS locks Claimed subset. |
| **FAIL** → **fixed** | `docs/performance.md` bars + claim table | Still tip ALIAS **0.5546** / REVERT **0.5000**; now cites G7 evidence. |
| **PASS** | `docs/workflows.md` | Thin human workflows; does not dump Specfold folklore (rule `80`). |
| **WARN** | `docs/completion.md` remaining-partials #2 | Wording “disposition follows sole G19 receipt” is slightly soft after G7 already closed partial — acceptable; left as-is after numeric tip note refresh. |

### 5. Naming / ADR / research note conventions

| Verdict | Cite | Note |
|---------|------|------|
| **PASS** | Recent research ids `20261111HHMMSS-….research.md` | Timestamp + kebab slug + frontmatter `id`/`rel` pattern holds. |
| **FAIL** → **fixed** | [[plan:root/20261111024440]] G_final row | Authored Hangul for “canon” violated [[sot:root/language-english-only]] / rule `05`. Replaced with **Canon**. |
| **PASS** | No new competing ADR invented for Hold-forever rows | Survey + gap research keep Non-goal / Impossible honest. |
| **WARN** | Historical research notes (e.g. [[research:root/20261111022345]]) | Correctly freeze tip-at-writing ratios (**0.5546**); readers must not treat as tip currency — residual/roadmap own tip. |

### 6. CI gates (fmt / clippy) hygiene

| Verdict | Cite | Note |
|---------|------|------|
| **PASS** | `.github/workflows/ci.yml` | `cargo fmt --check` + `clippy -D warnings`; `RUSTFLAGS: -D warnings`; host CI labeled advisory vs clean-room. |
| **WARN** | `1590fa2`, `b7720d9` after `7c172f6` | G-ladder land needed immediate rustfmt/clippy fix commits. Convention expects scoped fmt/clippy with the slice (`AGENTS.md` Done means). Prefer pre-push local lint; pattern is recurring but repaired quickly — not a green-fake. |
| **PASS** | No invent PASS in CI | Clean-room smoke job remains advisory; secrets-gated forge workflows separate. |

### 7. Forbidden patterns

| Verdict | Cite | Note |
|---------|------|------|
| **PASS** | `OWNER.md` | **Absent** at tip (no OWNER.md). |
| **PASS** | Tip commits + [[research:root/20261111030858]] | Explicit **no invent PASS**, **no G19 thrash**, **no unauthorized L9**. |
| **PASS** | G7 evidence | Exactly **one** sole Docker G19; no second remasure after ALIAS stay-na. |
| **PASS** | Force-push | No tip commit instructing force-push; safety rule `00` intact. |
| **WARN** | `AGENTS.md` router honesty paragraph | Lagged (“ALIAS/REVERT `aggressive_na`” + old evidence id) vs tip. **Fixed** pointer to [[evidence:root/20261111025200]]. |
| **FAIL** → **fixed** | `.agents/rules/80-public-surface.md` | Still taught “Pf5 partial **7/8** (CLONE-LOCAL NA)” and “OpenPGP N/A” — superseded by tip Claimed(partial) + G38 Claimed. |

### 8. Router / CONTRIBUTING / public-surface SoT

| Verdict | Cite | Note |
|---------|------|------|
| **PASS** | `CONTRIBUTING.md` | Thin pointer to human `docs/` + `AGENTS.md`; English; no invent PASS. |
| **PASS** | [[sot:root/public-surface]] §31–32 | Partial / live-forge honesty matches PARTIAL tip narrative. |
| **WARN** | [[sot:root/public-surface]] `updated` frontmatter | `2026-11-10T15:18:16Z` older than G-ladder land; body text largely refreshed earlier — recommend bumping `updated` when next substantive public-surface edit lands (not required for this audit). |

## Overall scorecard

| Area | Score |
|------|-------|
| Specfold-first before Claimed | **PASS** (with host-vs-Verified **WARN** on G3–G6) |
| Residual register tip currency | **FAIL** → **fixed** (PF5 rows + Verification §3) |
| Evidence packs present | **PASS** |
| Public docs vs tip | **FAIL** → **fixed** (limits / contribute / performance) |
| English / naming | **FAIL** → **fixed** (plan Hangul); research naming **PASS** |
| CI fmt/clippy | **PASS** with process **WARN** |
| Forbidden patterns | **PASS** (rule-80 + AGENTS pointer fixed) |

**Aggregate:** tip process is largely Specfold-honest and anti-thrash; the main compliance debt was **currency lag** (residual §F PF5 numbers, thin public filter/perf copy, stale rule-80 Pf5 7/8) after a large G-ladder land — not invent-PASS or OWNER folklore.

## Fixes applied in this change set

1. Residual register v**0.9.10** — FW-PF5-NA / J-ALIAS / J-REVERT + tip-currency row + Verification §3 → G7 evidence.
2. `docs/limits.md`, `docs/contribute.md` — filter + LFS locks honesty.
3. `docs/performance.md`, `docs/completion.md` — tip ALIAS/REVERT ratios + G7 cite.
4. Plan G_final row — **Canon** (English).
5. Rule `80-public-surface.md` + `AGENTS.md` PF5 pointer refresh.

## Recommendations (not fixed here)

1. **Clean-room twin for G3–G6** (or SoT-explicit host-Claimed deferral) so G_final “e2e for new Claimed surfaces” is not host-only.
2. **Pre-land lint habit** — run `cargo fmt` + `clippy -D warnings` before the feature commit to avoid fmt/clippy follow-up commits on `main`.
3. Optionally bump [[sot:root/public-surface]] frontmatter `updated` on next public-surface edit.
4. Leave historical research tip baselines frozen; teach agents to read **roadmap tip table + residual §F** for currency (already implied by Specfold discipline).

## Honesty

Research / audit only plus small doc/SoT drift fixes. **No** G19 remasure, **no** invent PASS, **no** L9 publish, **no** feature-crate conflict with parallel implementer.
