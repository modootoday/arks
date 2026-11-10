---
id: sot:root/agent-governance
title: Agent governance
status: active
created: 2026-11-08T08:20:00Z
updated: 2026-11-10T19:15:00Z
tags: [agents, governance, specfold, conventions, language, public, triad, classification, spec-local, m24, g28, deferred-zero, l3]
verify: manual
rel:
  depends_on:
    - sot:root/conventions-inherit
    - sot:root/language-english-only
    - sot:root/product-naming
    - sot:root/git-compat-surface
    - sot:root/product-verification-clean-room
    - sot:root/structural-conventions
    - sot:root/public-surface
    - sot:root/program-roadmap
    - sot:root/production-residual-register
    - sot:root/storage-celir-boundary
    - sot:root/celir-index-dual-path
    - sot:root/interop-test-matrix
    - sot:root/agent-dev-readiness
    - sot:root/product-intent
    - sot:root/arks-hub-product-intent
    - sot:root/arks-daemon-product-intent
    - sot:root/arks-agent-product-intent
    - research:root/20261111034556
    - research:root/20261111035700
  governs:
    - AGENTS.md
    - .agents/
    - .spec/README.md
---

# Agent governance

Version **0.3.0**. Normative intent for how AI agents use **AGENTS.md**, `.agents/**`, `.spec/**`, and Specfold in this **public** repository. Product semantics remain owned by domain SoTs; this SoT owns **process routing** and **spec classification**.

Human-facing index: `.spec/README.md`. Rule: `.agents/rules/25-spec-classification.md`.

## Rule

### AGENTS.md is a router, not a second SoT

1. Root `AGENTS.md` MUST stay short: mission, hard constraints, progressive disclosure table, Done means, ambiguity.
2. Normative product/compat/verification intent MUST live in `.spec/sot/` (and cited research). Agents MUST NOT treat AGENTS prose as a competing product SoT.
3. When AGENTS and a SoT disagree, the **owning SoT wins**; fix the router link rather than expanding AGENTS into a ledger.

### Required trees

4. Agent governance for arks REQUIRES:
   - `AGENTS.md` (instructions asset)
   - `.agents/rules/` (always-on numbered rules)
   - `.agents/skills/` (SKILL.md with “Use when…” descriptions)
   - Optional `.agents/agents/` role perspectives
   - `.spec/README.md` (public classification index)
5. These trees are part of Specfold design canon for this repo (see [[sot:root/conventions-inherit]]). Register them under `spec.toml` `[[assets]]` so `spec assets` / rules A1–A2 apply.

### Public-repo classification (normative)

6. **`.spec/` is tracked public brain** — SoT / plan / ADR / research / scrubbed evidence. Assume world-readable.
7. **`.spec.local/` is machine-private** — configured in `spec.toml` `[local] path`; **MUST** stay gitignored; NEVER commit secrets, agent transcripts, pin extracts, or host dumps there into tracked trees.
8. **`.agents/` is tracked process** — rules/skills/role perspectives only; not a dump for SoT ledgers or raw evidence.
9. **`.specfold/trash/`** is Specfold delete backup (under gitignored `.specfold/`) — do not resurrect as active Claimed without a deliberate SoT amend.
10. Classification table (short form; full text in `.spec/README.md`):

| Bucket | Home | Commit? |
|--------|------|---------|
| SoT / plan / ADR | `.spec/sot|plan|adr/` | Yes |
| Research | `.spec/research/` | Yes (`active` or `archival`) |
| Evidence (scrubbed) | `.spec/evidence/` | Yes |
| Residual honesty | `.spec/sot/production-residual-register.sot.md` | Yes |
| Local scratch | `.spec.local/` | **Never** |
| Trash | `.specfold/trash/` | Never |

11. **No secrets** in tracked `.spec/` or `.agents/`. Evidence receipts MAY be public when non-secret. Agent dumps → `.spec.local/` or delete.
12. **Push deferred** while remote is empty / owner has not authorized a clean public history push ([[research:root/20261111041000]]). Agents MUST NOT `git push` origin unless the user explicitly orders it.

### Product shape (process honesty)

13. Agents MUST NOT reintroduce forge-as-core or other trimmed surfaces into active SoT Claimed language. Active cut ([[sot:root/product-intent]], `.spec/README.md`, tip ≥ `9df7fb9`):
    - **Single release / published CLI = `arks` only.**
    - Concept peers: **`git`** → ordinary `arks` cmds; **`git-daemon`** → **`arks daemon`** (internal crate `arks-daemon`); **`gh`** → **`arks hub …`** (internal crate `arks-hub`); **agent bots** → **`arks agent …`** (internal crate `arks-agent`).
    - Absolute: product paths never spawn `git` or `gh`.
    - Do **not** document or Claim separate `arks-hub` / `arks-daemon` / `arks-agent` product binaries/releases.
14. **CELIR** ObjectStore remains Refuse; optional `celir-index` is **ops/CI-only** (default OFF) — not product Claimed path after trim.
15. **G8 live SLO / OIDC release channels** are **ops/CI-only** — never end-user Claimed product DoD.
16. Historical ladders that treat over-design as required product completion MUST be marked `status: archival` or corrected in tip tables so they do not pollute active SoT.

### Hard constraints agents must honor

17. **English only** — [[sot:root/language-english-only]]; rule `.agents/rules/05-english-only.md`.
18. **Standards-first** — [[sot:root/git-compat-surface]].
19. **Clean-room authoritative** for product “verified” — [[sot:root/product-verification-clean-room]].
20. **No literal competing convention SoT** — cite humanymous live paths; pointer map in [[sot:root/conventions-inherit]].
21. **CELIR / lease boundary** — filesystem `.git` is the sole Git-compat **object store**. CELIR MAY appear only as optional index-only feature `celir-index` (default OFF) ([[sot:root/storage-celir-boundary]], [[adr:root/20261110052030]]). **ObjectStore via CELIR remains Refuse.** Lease/concurrency has its own SoT (rule `.agents/rules/50-celir-boundary.md`).
22. **Bottom-up build order** — inner modules first (rule `.agents/rules/70-bottom-up-build-order.md`; skill `implement-sot-slice`). Program ladder **M0–M26** lives in [[sot:root/program-roadmap]].
23. **Structural conventions** — [[sot:root/structural-conventions]]; rule `.agents/rules/15-structural-conventions.md`.
24. **Public surface** — [[sot:root/public-surface]]; rules `.agents/rules/80-public-surface.md` and `.agents/rules/25-spec-classification.md`.

### Current program honesty (router pointers — not a second ledger)

25. Agents MUST progressive-disclose live truth from owning SoTs rather than chat memory:
    - **Triad tip table** — [[sot:root/program-roadmap]] (post-trim honesty for host-env / LFS-product / forge / G8).
    - **Residual** — [[sot:root/production-residual-register]] (trimmed rows MUST NOT stay Claimed as product).
    - **Hub / daemon / agent design** — [[sot:root/arks-hub-product-intent]] / [[sot:root/arks-daemon-product-intent]] / [[sot:root/arks-agent-product-intent]] (design ≠ Claimed; same `arks` binary).
    - **Catalog Deferred=0** — Closed-deferred is not a lasting work-item disposition.
26. Prefer updating owning `.spec/` documents when process or bar truth changes — **no folklore** in chat-only process invention.

### Specfold loop

27. Before governed edits: `spec context <path>`. Prefer updating `.spec/` over chat folklore. Before docs/spec done: `spec check`.

### Ownership

28. This SoT does **not** own product-intent, crate-architecture, structural-conventions, cli-surface, core-object-model, interop-test-matrix content, storage-celir-boundary detail, celir-index-dual-path detail, lease-concurrency-intent, public-surface placement detail beyond classification cross-links, program-roadmap exits, residual-register rows, or agent-dev-readiness checklists. Those sibling SoTs own their domains; AGENTS may only progressive-disclose them.
29. Do not edit sibling domain SoTs from agent-governance work except to add reciprocal cross-links when those owners request them — **except** when correcting out-of-scope Claimed language to match the triad (doc cleanse), which MAY touch tip tables / residual dispositions / public-surface honesty with cites to trim research.

## Why

A public empty→later-push repo needs a clear `.spec` vs `.spec.local` vs `.agents` map so agents do not commit secrets, re-Claim trimmed over-design, or treat ops harnesses as product DoD. A thin router plus classification keeps multi-tool sessions aligned without a second product SoT.

## Verification

Manual:

1. `spec check` clean for this document and related assets.
2. `spec assets` lists instructions, rules, skills, and agents roots.
3. AGENTS.md contains no normative Git wire/object semantics that are absent from owning SoTs.
4. Rules/skills named in AGENTS exist on disk (including `25-spec-classification`).
5. `.gitignore` excludes `.spec.local/` and `.specfold/`.
6. Progressive-disclosure pointers name the triad, M0–M26 historical ladder, residual honesty after trim, and push-deferred note without dumping gate catalogs into AGENTS.
