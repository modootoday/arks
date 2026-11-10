---
id: sot:root/agent-dev-readiness
title: Agent dev readiness
status: active
created: 2026-11-08T08:21:48Z
updated: 2026-11-09T21:25:00Z
tags: [agent, readiness, specfold, dod, m24, g28, deferred-zero, l3, celir-index]
verify: manual
rel:
  depends_on:
    - sot:root/product-intent
    - sot:root/crate-architecture
    - sot:root/structural-conventions
    - sot:root/core-object-model
    - sot:root/storage-celir-boundary
    - sot:root/celir-index-dual-path
    - sot:root/lease-concurrency-intent
    - sot:root/git-compat-surface
    - sot:root/remote-host-compatibility-surface
    - sot:root/product-verification-clean-room
    - sot:root/cli-surface
    - sot:root/interop-test-matrix
    - sot:root/program-roadmap
    - sot:root/production-residual-register
    - sot:root/public-surface
    - sot:root/clean-room-journey-compare
    - sot:root/product-naming
    - sot:root/agent-governance
    - sot:root/language-english-only
    - sot:root/conventions-inherit
    - plan:root/20261108082148
    - plan:root/20261108082110
    - research:root/20261108073650
    - research:root/20261108071859
---

# Agent dev readiness

Version **0.2.0**. Definition of Done for **“an AI can start implement/verify from specs alone”** in this repository. Language lock: **all project documents and agent-facing text are English-only** (same top-level principle as humanymous). This SoT does not replace product SoTs; it states when the Specfold + agent-router set is sufficient to begin bottom-up work without inventing process or Git semantics.

## Rule

### Language

1. **English-only.** Specfold bodies, `AGENTS.md`, `.agents/**` rules/skills, plans, and research MUST be written in English. Non-English may appear only as cited proper nouns or collision notes inside research evidence (not as normative instructions).

### Required SoT set (minimum)

An agent MAY claim “spec-ready to implement” for a slice only when the following are present and `spec check` is clean (or residuals are explicitly documented):

| ID | Role |
|----|------|
| [[sot:root/product-naming]] | Product / crate / bin lock (`arks`) |
| [[sot:root/product-intent]] | Goals, non-goals, success (inner lane) |
| [[sot:root/crate-architecture]] | Inner→outer crate dependency rule |
| [[sot:root/structural-conventions]] | SRP, layout, naming, anti-fragmentation, Cargo-standard dirs |
| [[sot:root/core-object-model]] | Object/ref/identity aligned with git-compat |
| [[sot:root/storage-celir-boundary]] | CELIR ready/not-ready; FS `.git` sole ObjectStore; optional `celir-index` only |
| [[sot:root/celir-index-dual-path]] | M24 index-only dual-path companion (ObjectStore Refuse) |
| [[sot:root/lease-concurrency-intent]] | Lease vs Git locks; L3 claimed shape |
| [[sot:root/git-compat-surface]] | Normative Git format/protocol intent |
| [[sot:root/remote-host-compatibility-surface]] | GitHub-class remote (A)/(B) |
| [[sot:root/product-verification-clean-room]] | Docker clean-room authority |
| [[sot:root/cli-surface]] | Porcelain / alias surface |
| [[sot:root/interop-test-matrix]] | Gate unlock ladder (**G0–G27**; +G28 when present) |
| [[sot:root/program-roadmap]] | Milestones **M0–M24** and production exits |
| [[sot:root/production-residual-register]] | Post-bar residual honesty; **Deferred=0**; no bare development Opens |
| [[sot:root/public-surface]] | Human vs agent vocabulary; no secrets; Pages thin |
| [[sot:root/clean-room-journey-compare]] | G16 journey catalog + timing honesty |
| [[sot:root/agent-governance]] | AGENTS router + `.agents/**` process canon |
| [[sot:root/language-english-only]] | English-only authored prose lock |
| [[sot:root/conventions-inherit]] | humanymous cite map (no forked ledger) |
| [[plan:root/20261108082148]] | Whole-product bottom-up phases (**C→S→R→P→L→H→E** + post-E program ladder) |
| [[plan:root/20261108082110]] | Core build slice detail |

Supporting research under `.spec/research/*-git-*.research.md` and [[research:root/20261108073650]] MUST remain cited by the SoTs above. Process trees (`AGENTS.md`, `.agents/rules|skills`) are governed by [[sot:root/agent-governance]] — not optional folklore.

### How to start a change (machine steps)

2. **Context first.** Run `spec context <path>` for the code or doc path you will touch, or `spec show <id>` / read the owning SoT from the table above.
3. **Find canon.** `spec find --type sot` (or `spec find <words>`) to locate governing documents; prefer SoT IDs over chat memory.
4. **Check before done.** `spec check` after Specfold edits; fix errors in owned docs. Document residual warnings you do not own.
5. **Verify venue.** Product interop/acceptance claims follow [[sot:root/product-verification-clean-room]] and Gate IDs in [[sot:root/interop-test-matrix]] — host greens are advisory. Heavy Docker gates (G16–G27 and any live-forge opt-in) MUST respect sole-runner honesty — do not interrupt a sibling live-GH agent.
6. **Build order.** Implement/verify inner-first per [[plan:root/20261108082148]] and rule `.agents/rules/70-bottom-up-build-order.md`; program exits follow [[sot:root/program-roadmap]] (**M0–M24**).

### Claim / session light rules (pointer only)

7. **Do not fork a second session ledger here.** Light overlap rules live in `.agents/rules/90-session-overlap.md`. Richer multi-provider board / git-coord patterns are **cite-only** from humanymous: `C:\workspaces\humanymous\.agents\rules\90-session-overlap.md` and `C:\workspaces\humanymous\.agents\sessions\`.
8. Before non-trivial multi-file edits: check for conflicting path ownership; prefer disjoint claims; stop and ask on conflict.
9. Do not rewrite sibling-owned SoTs beyond requested cross-links (inner core vs outer surface vs governance).

### Ready vs not ready

**Ready (DoD checklist)**

- [ ] Required SoT set above exists with English bodies (not empty Rule/Why/Verification stubs for the slice being implemented).
- [ ] [[plan:root/20261108082148]] phases name inputs, outputs, clean-room Gate IDs, exit criteria; [[sot:root/program-roadmap]] covers **M0–M24**.
- [ ] `spec check` reports **0 errors** on the working tree’s Specfold docs (warnings explicitly owned/documented).
- [ ] Agent can name the next phase / Gate ID without inventing Git semantics; knows **G17–G27** (and **G28** when present).
- [ ] Agent states current honesty: L3 **Claimed** shape; shared-index **M26 / G34 Claimed**; CELIR **index-only** (`celir-index`, ObjectStore Refuse); catalog **Deferred=0**; M25 G29–G32 Claimed; W1–W5 residual Claimed with partial honesty (filter N/A, Pf5 7/8, OpenPGP N/A); no invented live PASS.
- [ ] `AGENTS.md` + `.agents/rules` present for safety, Specfold discipline, structural conventions, clean-room, CELIR boundary, bottom-up order, public surface, session overlap.
- [ ] Ambiguity: ask the user when SoTs conflict or Open decisions block the claimed gate (`.agents/rules/40-ambiguity-ask.md`).

**Not ready**

- Empty stub SoTs for the slice under work.
- Claiming remote/CLI verified without clean-room Gate unlock.
- Inventing CELIR ObjectStore APIs, enabling `celir-index` by default, or marketing “CELIR-backed Git” without G23 receipt + FS sole ObjectStore wording.
- Treating Closed-deferred as a lasting success exit (M21 requires **Deferred=0** terminal dispositions).
- Inventing live-forge journey PASS without opt-in secrets + receipt.
- Korean or other non-English normative instructions in project docs.

## Why

Greenfield agents fail by inventing process, reordering layers, or treating host Git as proof. A single readiness SoT ties Specfold discovery (`context` / `check`), the required document set, bottom-up phasing, current M0–M24 / G0–G27 honesty, and light session rules so implementation can start from the tree alone. English-only keeps humanymous-aligned multi-agent collaboration readable.

## Verification

1. `spec check` clean for this SoT and its `depends_on` targets (or residuals listed in a plan/audit note).
2. Spot-check: from a cold checkout, `spec find sot` lists the required set; `spec show sot:root/agent-dev-readiness` exposes this checklist.
3. Governance lane keeps `AGENTS.md` / `.agents` pointers accurate; this SoT only cites them.
4. When stub inner SoTs gain full bodies, re-run readiness mentally against the checklist before coding that slice.
