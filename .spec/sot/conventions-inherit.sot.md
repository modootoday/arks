---
id: sot:root/conventions-inherit
title: Conventions inherit (humanymous pointer map)
status: active
created: 2026-11-08T08:20:00Z
updated: 2026-11-08T08:55:00Z
tags: [conventions, humanymous, specfold, agents, language, public]
verify: manual
rel:
  depends_on: [sot:root/product-naming]
---

# Conventions inherit

Version 0.1.0. Normative **pointer map**: arks inherits Specfold / agent-layout conventions from **humanymous** by live cite. This SoT forbids copying mega ledgers into arks as a competing convention Source of Truth.

## Rule

### Cite, do not fork

1. When an agent needs Specfold usage patterns, AGENTS progressive-disclosure style, skill/rule/agent layout, bottom-up structural layers, Docker completion-authority affinity, or session-overlap patterns, it MUST open the humanymous path below (or the arks thin delta that cites it).
2. arks MAY record **thin deltas** (product naming, Git-compat, clean-room intent, CELIR boundary, bottom-up order for Git modules) in arks SoTs/rules. arks MUST NOT paste humanymous structural ledgers, session boards, or Rust-migration tables wholesale.
3. If a needed convention is missing in both trees, **ask** (`.agents/rules/40-ambiguity-ask.md`) — do not invent a third convention dialect.

### Pointer map (live)

Base: `C:\workspaces\humanymous`

| Concern | Humanymous path | arks thin surface |
|---------|-----------------|-------------------|
| Root AGENTS style | `AGENTS.md` | `AGENTS.md` (router) |
| Safety | `.agents/rules/00-safety.md` | `.agents/rules/00-safety.md` |
| English-only authored prose | `.agents/rules/10-rust-conventions.md` (English paragraphs); `.spec/sot/rust-migration.sot.md` (authored docs/comments English) | [[sot:root/language-english-only]] + `.agents/rules/05-english-only.md` |
| Rust conventions | `.agents/rules/10-rust-conventions.md` + `.spec/sot/rust-migration.sot.md` | `.agents/rules/10-rust-conventions.md` (cite only) |
| Ambiguity | `.agents/rules/40-ambiguity-ask.md` | `.agents/rules/40-ambiguity-ask.md` |
| Docker completion authority | `.agents/rules/60-e2e-docker-only.md`, `96-docker-engine-affinity.md`; skill `red-blue-validate` | `.agents/rules/60-clean-room-verify.md`; skill `verify-clean-room`; [[sot:root/product-verification-clean-room]] |
| Bottom-up layers | `.spec/sot/structural-development.sot.md` | `.agents/rules/70-bottom-up-build-order.md`; skill `implement-sot-slice` |
| Structural / SRP / layout (thin arks lock) | `.spec/sot/crate-architecture.sot.md`; `.spec/sot/rust-migration.sot.md` (Structure and naming); `.spec/sot/structural-development.sot.md` (`src/<context>`) | [[sot:root/structural-conventions]] + `.agents/rules/15-structural-conventions.md` |
| Session overlap | `.agents/rules/90-session-overlap.md`, `.agents/sessions/` | `.agents/rules/90-session-overlap.md` (light) |
| Design / implement skills | `.agents/skills/design-sot`, `implement-sot-slice` | same skill names under `.agents/skills/` (arks-adapted) |
| Specfold project entry | `AGENTS.md` Specfold-first section | [[sot:root/agent-governance]] + skill `specfold-ops` |
| Public surface / Pages vs Specfold | (arks-specific; no humanymous fork) | [[sot:root/public-surface]] + `.agents/rules/80-public-surface.md` |

### Specfold assets

4. Declare agent trees in `spec.toml` via `[[assets]]` (`kind` = `instructions` | `rule` | `skill` | `agent`) so `spec assets` and checks A1/A2 apply. Do not rely on chat folklore for asset discovery.

### Non-goals

- Replacing humanymous as the convention laboratory.
- Porting humanymous detection-freeze / red-blue wargame product rules into arks.
- Authoring lease/CELIR product SoTs here (boundary only until those SoTs exist).

## Why

arks needs agents that can implement/verify without inventing process, while remaining a Git-compat product with its own SoTs. Pointer inheritance keeps one convention brain (humanymous) and one product brain (arks `.spec/sot`).

## Verification

Manual:

1. Pointer paths above exist on the humanymous workspace used by agents (or documented offline).
2. arks rules/skills that claim “cite humanymous” name concrete paths, not vague “follow humanymous.”
3. `spec check` clean; no duplicate mega structural SoT appears under arks `.spec/sot/` that restates humanymous L0–L5 ledgers.
