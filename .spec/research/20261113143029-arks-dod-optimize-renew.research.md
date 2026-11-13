---
id: research:root/20261113143029
title: Optimize / renew three-pillar DoD — cut Partial thrash, bounded raises only
status: active
created: 2026-11-13T05:30:29Z
updated: 2026-11-13T05:30:29Z
tags: [research, dod, symmetry, optimize, no-invent-claimed, git, daemon, hub, serve]
verify: manual
rel:
  depends_on:
    - sot:root/verify-cleanroom-symmetry-matrix
    - sot:root/production-residual-register
    - sot:root/arks-claimed-residual-gates
    - sot:root/arks-native-e2e-checklist
    - sot:root/arks-cli-personality-factory
    - sot:root/arks-hub-cli-surface
    - sot:root/arks-daemon-product-intent
    - sot:root/arks-self-host-server
    - sot:root/arks-in-scope-leftovers-wave
  related:
    - sot:root/arks-dod-optimized
    - research:root/20261111135200
    - research:root/20261111123556
---

# Optimize / renew three-pillar DoD — research note

English-only. Tip surveyed: **`ea6571c`**. Design ≠ Claimed until owning SoT + Docker receipts.

## Problem

Old public-story DoD ([[sot:root/verify-cleanroom-symmetry-matrix]]) is already met at tip: **CLI / daemon / hub v1 Missing=0**. Agents still thrash treating **Partial×117**, hub **terminal Partial×6**, hub **later×119**, OUT Refuse, SysV, L9/G8/Impossible, and full unified-overlap as “maybe DoD.” That invites stub-stampede, not product completeness.

Owner correction: goal is **git / git-daemon / gh symmetry** (+ related serve), **not** exhausting every Partial to Claimed.

## Optimization criteria

1. Fewer ambiguous Partials-as-maybe-DoD.
2. Sharper PASS/FAIL under `ARKS_DOD_STRICT=1` (Linux Docker pin Git **2.39.5**).
3. Less token thrash — no invent Claimed; no origin push.
4. Still three pillars + related serve / residual Claimed gates already evidenced.

## Cut / reclassify (explicit non-DoD)

| Surface | Disposition |
|---------|-------------|
| Hub `phase=later` Deferred **×119** | **OUT of DoD** — not v1 |
| Hub OUT Refuse/N/A (`extension`/`copilot`/…) | **OUT** |
| CLI flag-depth Partial **×117** remainder | **OUT of DoD** — named/survey Claimed subsets stay **carrying** evidence only; remainder honest Partial ≠ must-pass |
| Hub terminal Partial **×6** (`auth refresh`, `pr diff`/`checks`/`update-branch`/`revert`, `repo fork`) | **Keep terminal** — do **not** promote (OAuth/CI/git-object; invent forbidden) |
| Full unified personality overlap depth | **OUT** beyond bounded OD-UNIFIED smoke |
| SysV classic double-fork | **Refuse terminal** — peer is single-fork+`setsid` |
| Impossible / L9 / live G8 latency SLO | **OUT** — residual next-queue / secrets / Impossible |

## Raise only if product-complete + evidence-feasible (no invent)

| ID | Bar | Why feasible now |
|----|-----|------------------|
| **OD-UNIFIED** | Bounded `arks` personality overlap STRICT arm: default unified honesty (Git section + Forge skip note) + `--git-only` / `--forge-only` / `--porcelain` mode flags | Code already in `OverlapFacade`; checklist noted “no dedicated STRICT arm” |
| **OD-SERVE-CFG** | `arks serve config` smoke Claimed: `path`/`show`/`set`/`validate` exit 0 with honest Partial schema labels | Product path already implemented Partial; hosting e2e lacked smoke |
| **OD-COMMIT-HONESTY** | `arks commit` without `-m`/`-F`/`-t` and without editor env → non-zero honesty; with scripted `GIT_EDITOR` → writes `COMMIT_EDITMSG` | O3/FS1 wire already in `cmd/core.rs`; needs STRICT recipe receipt |

Do **not** raise: stampede CLI Partial×117; invent hub terminal×6 Claimed; WAN-secure-by-default; SysV; L9; G8 live SLO.

## Renewed DoD floor (preview)

Must-pass = three-pillar Missing=0 + STRICT greens + related self-host / B-gate Claimed already on tip **plus** OD-UNIFIED / OD-SERVE-CFG / OD-COMMIT-HONESTY when evidenced. Everything in the cut table is **explicitly non-DoD**.

Normative design: [[sot:root/arks-dod-optimized]].
