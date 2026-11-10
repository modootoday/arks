---
id: sot:root/language-english-only
title: Language — English only
status: active
created: 2026-11-08T08:25:00Z
updated: 2026-11-11T06:15:00Z
tags: [language, english, hangul-ban, conventions, agents, public, hooks]
verify: manual
rel:
  depends_on: [sot:root/conventions-inherit]
---

# Language — English only

Version 0.1.1. Top-level principle for **arks**: all authored project documents and text are written in **English only**. The same policy is used by humanymous; this SoT records the arks lock and cites the live humanymous paths.

**Hangul ban (clarification):** Korean/Hangul operator briefings are chat-only. Tracked Specfold, `docs/`, `AGENTS.md`, `.agents/**`, and commit/PR prose MUST stay English. Always-on rule: `.agents/rules/05-english-only.md`.

## Rule

### Scope (MUST be English)

1. Specfold documents under `.spec/` (SoTs, plans, research authored prose, evidence summaries, change notes).
2. Root `AGENTS.md` and all of `.agents/**` (rules, skills, agents, sessions if present).
3. Public documentation under `docs/` and GitHub Pages output sourced from it.
4. Source code comments and docstrings.
5. Commit messages and pull-request titles/bodies authored for this repository.
6. Workflow labels, README files, and other authored explanatory prose in-tree.

### Allowed non-English or non-prose bytes

7. **Proper nouns and product identifiers** may remain as locked: `arks`, `CELIR`, `humanymous`, crate/bin names, host names, and similar.
8. **Immutable raw evidence**, third-party quotations, literal UI titles, and intentional multilingual **test fixtures** may retain original language; they are source data, not authored explanatory prose. Prefer romanization or English paraphrase when an authored research note needs to discuss foreign phonetics (do not embed Hangul/other scripts in authored narrative unless quoting immutable evidence).
9. **Unicode** (symbols, punctuation, math) is not by itself a language violation.
10. **Conversation language** with operators does not waive this SoT for in-repo authored text.

### Cite (humanymous — do not fork ledger)

| Concern | Live path |
|---------|-----------|
| English authored prose (rules) | `C:\workspaces\humanymous\.agents\rules\10-rust-conventions.md` (opening English-only paragraphs) |
| English authored docs/comments (SoT) | `C:\workspaces\humanymous\.spec\sot\rust-migration.sot.md` (authored documentation / source comments must be English) |
| Optional tooling affinity | humanymous `language-check` / language-preservation evals under `.agents/evals/` — cite if/when arks adopts equivalent tooling |

### Agent surfaces

11. Always-on rule: `.agents/rules/05-english-only.md`.
12. Router: root `AGENTS.md` hard constraint **English only** → this SoT.
13. Pointer row: [[sot:root/conventions-inherit]].
14. Public Pages / README / crate metadata English lock also sits under [[sot:root/public-surface]] (placement + secrets); this SoT owns the language rule itself.
15. **Hooks (process aids):** `.cursor/hooks/hangul-docs-guard.ps1` watches `.spec/`, `docs/`, `AGENTS.md`, `.agents/`, README/CONTRIBUTING/SECURITY/LICENSE*, `verify/`, `.cursor/`, `.github/` after edits; `.cursor/hooks/governance-shell-guard.ps1` denies Hangul in `git commit` messages and staged watched paths. Chat Korean remains allowed.

## Why

Mixed-language SoTs and agent instructions raise the cost of multi-provider collaboration and diverge from humanymous convention. One language for authored prose keeps Specfold, AGENTS, and public docs reviewable without translation debt.

## Verification

Manual (until a language-check tool is adopted):

1. `spec check` clean for this document.
2. Spot-scan for Hangul / other authored non-English prose in `.spec/`, `AGENTS.md`, `.agents/`, `docs/`.
3. New findings are either converted to English or recorded as preservation debt with an explicit reason (immutable evidence / third-party quote / test fixture).
