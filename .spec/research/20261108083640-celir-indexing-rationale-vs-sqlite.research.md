---
id: research:root/20261108083640
title: CELIR indexing rationale vs SQLite
status: active
created: 2026-11-08T08:36:40Z
updated: 2026-11-08T10:21:00Z
rel:
  depends_on: [sot:root/storage-celir-boundary, sot:root/product-intent, evidence:root/20261108192100]
---

# CELIR indexing rationale vs SQLite

## Question

Why does arks intend **CELIR** (`C:\workspaces\discrete-circuit`) as the alternate storage lineage, and how should that rationale be separated from arks readiness / blocked work?

## Method

1. Treat the **local** discrete-circuit working tree as authoritative (no git remote configured on that tree at research time). Do not use GitHub/remote as SoT.
2. Read owning boundary SoT [[sot:root/storage-celir-boundary]] and product storage goal in [[sot:root/product-intent]].
3. Inventory capability from discrete-circuit docs at catalog pin **`b30f5af6ff1131e75ac7b1c4755f97ea01b6deaa`** (prior `725c4e0a…` stale; see [[evidence:root/20261108192100]] and [[sot:root/storage-celir-boundary]] v0.5.0) plus local `README.md`, `AGENTS.md`, package/crate docs, and roadmap — cite paths; do not invent APIs.
4. Use CELIR session transcripts only as a status proxy when live cross-session ask is unavailable.
5. Do **not** treat dirty WIP as freeze-ready API.

## Findings

1. **Selection reason (arks product intent):** CELIR is chosen because the local CELIR project has **established index-build performance advantages versus SQLite** for the workloads it qualifies. Primary local evidence:
   - `C:\workspaces\discrete-circuit\crates\celir-index-runtime\.spec\artifacts\initialized-composition-whole-20261026.md` (70.5M-row / 39-file composition; CELIR median construction **67.8479836s** vs native SQLite **91.9787101s**; ratio of medians **0.7376487834**; open/query mostly slower).
   - Summary: `C:\workspaces\discrete-circuit\README.md` (performance paragraph).
   - Continuity gate: maintain verified large-index CELIR/SQLite build-time ratio **&lt; 0.9** on qualified recipes.
2. **Claim limits (normative in boundary SoT):** Build win is **recipe-scoped only**. Open/most nonempty queries remain slower on the same whole-data gates. **Do not merge** the separate small exact-lookup DCI4 cohort with whole-data claims. See Index vs SQLite table in [[sot:root/storage-celir-boundary]].
3. **Capability catalog (local, pinned):** Areas at pin `b30f5af6…` (workspace, codecs/inputs, index + runtime, SQL/FFI, Android blocked, celir-db, graph, learning Refuse M8, resource-lifecycle, Docker matrix, storage gate OPEN, M1/M2 campaign blocked, **VCS object-store Refuse**) are recorded in [[sot:root/storage-celir-boundary]] Appendix A. Package map: `C:\workspaces\discrete-circuit\AGENTS.md`. Receipt: [[evidence:root/20261108192100]].
4. **May call soon vs must wait:** Codecs/file-region/range-download, index build + immutable-root read (single-writer), resource-lifecycle, and finite Memory DB/graph CRUD map to arks **Phase B** lab only. Learning Refuse/M8, M1/M2 Seal as Git-compat deps, Schema JNI/Android/Win/macOS, power-loss durability, universal query superiority, dirty-tree API freeze, and any CELIR VCS put/get/exists API remain **must wait / Refuse**.
5. **Locks closed (v0.5.0):** Consumer facade Refuse (VCS freeze) / lab Answered; VCS object-store **Refuse**; durability process-exit only; concurrency CELIR single-writer + **arks-lease owns multi-writer**; recovery identity non-collapse; resource ownership; delivery; campaign decoupling. Residual opens: Phase B scheduling, Phase C subset, power-loss close path, future CELIR VCS contract.
6. **Coordination / remaining constraints (local):** `C:\workspaces\discrete-circuit\packages\celir-rules\.spec\plans\lightweight-stack-roadmap.md`; adoption refuse ADR `...\adrs\learning-model-adoption.md`; storage gate `...\handoffs\cursor-storage-gate.md` (**OPEN** — does not clear M1/M2/learning Refuse).
7. **Local-only caveat:** discrete-circuit has no remote; dirty WIP present. Catalog pin is `b30f5af6…`; tree may advance — re-read local paths before claiming flips.
8. **Cross-session:** Transcript proxy at `C:\Users\smile\.cursor\projects\c-workspaces-discrete-circuit\agent-transcripts\32f017f7-9c49-41e8-a99e-b8237cd88005\32f017f7-9c49-41e8-a99e-b8237cd88005.jsonl` (subordinate to on-disk docs).
9. **Why ≠ ready:** Index-build rationale and Phase B lab eligibility do **not** authorize silent replacement of the filesystem Git object store, inventing CELIR APIs in arks, claiming a CELIR VCS object-store contract, or claiming CELIR-backed verification. See readiness + Phase C gates in [[sot:root/storage-celir-boundary]].
