---
id: evidence:root/20261108192100
title: CELIR boundary response receipt
status: active
created: 2026-11-08T10:21:00Z
updated: 2026-11-08T10:21:00Z
tags: [celir, storage, boundary, inbound, pin, refuse]
rel:
  depends_on:
    - sot:root/storage-celir-boundary
    - sot:root/lease-concurrency-intent
    - research:root/20261108083640
    - plan:root/20261108090653
---

# CELIR → arks storage boundary response receipt

## Context

Inbound **document** response from the local CELIR authority tree, canonicalized into [[sot:root/storage-celir-boundary]] **v0.5.0**. Design-only; no CELIR path-dep activation; no clean-room product claim.

## Authority snapshot

| Field | Value |
|-------|--------|
| Tree | `C:\workspaces\discrete-circuit` |
| **Pinned HEAD** | `b30f5af6ff1131e75ac7b1c4755f97ea01b6deaa` |
| Prior pin (stale) | `725c4e0ad9f476616ce6d4385917b9e7c28adbd3` |
| Branch | `master` |
| Remote | none (local-only) |
| Dirty WIP | present — **not** freeze |
| Session proxy | `…\agent-transcripts\32f017f7-9c49-41e8-a99e-b8237cd88005\…jsonl` (subordinate to on-disk docs) |

## Non-negotiables acknowledged (CELIR → arks)

1. No silent replace of filesystem Git object/ref/pack/index in Git-compat mode.
2. No CELIR API invent/smuggle in arks Rings 0–5; only behind `arks-store`.
3. Dirty WIP / storage gate OPEN / learning Refuse ≠ API freeze or readiness.
4. SQLite compare = recipe-scoped index-build only; no open/query universal superiority.
5. No incremental VCS append on append-only index (implemented, not adopted).
6. No invented multi-writer; if CELIR does not provide it → `arks-lease` / external coordination.
7. CELIR authority = local discrete-circuit tree only.

## Locks closed (summary)

| # | Topic | Status |
|---|-------|--------|
| 1 | Consumer facade | **Refuse** VCS product freeze; lab-only surfaces listed |
| 2 | VCS object-store contract | **Refuse** — no put/get/exists Git API on CELIR roadmap |
| 3 | Durability | **Answered** — process-exit barriers only; power-loss Forbidden |
| 4 | Concurrency | **Answered** — CELIR single-writer; **arks-lease owns multi-writer** |
| 5 | Recovery identity | **Answered** — do not collapse generation / PersistentIdentity / schema / Ready |
| 6 | Resource ownership | **Answered** — CELIR Scratch vs arks product dirs |
| 7 | Delivery | **Answered** — path-dep lab ≠ Actions release; Docker pass ≠ release |
| 8 | Campaign coupling | **Answered** — learning/M1/M2 independent of Git-compat |
| 9 | Catalog @ pin | **Answered** — capability surface @ `b30f5af6…` |

## Phase B / C (summary)

- **Phase B may-call:** file-region, byte-codec, sorted-run, range-download; index/index-runtime under single-writer; resource-lifecycle; finite DB Memory/Process/Graph lab — behind `arks-store` only; **not** frozen VCS API.
- **Phase C:** explicit subset + clean-room + Git-compat OID/pack/ref preserved + durability ≤ process-exit + arks-lease multi-writer + recovery identity honesty + no learning/storage-gate/WIP as readiness.

## Residual opens (arks)

Phase B scheduling; Phase C subset selection; power-loss close path (CELIR-owned); future CELIR-authored VCS contract (absent).

## Cite (paths under discrete-circuit — do not paste primary text)

`README.md`; `packages/celir-rules/.spec/plans/lightweight-stack-roadmap.md`; `packages/celir-index/.spec/adrs/0003-index-lifecycle.md`; `packages/celir-index/.spec/sots/snapshot-store.md`; `packages/celir-db/.spec/sots/process-persistence.md`; `crates/celir-index-runtime/.spec/plans/append-only-index.md`; `crates/celir-index-runtime/.spec/artifacts/initialized-composition-whole-20261026.md`; `packages/celir-rules/.spec/adrs/learning-model-adoption.md`; `.agents/rules/release-delivery.md`; `crates/celir-cli/.spec/artifacts/docker-product-matrix.md`; `.agents/rules/experiment-storage.md`; `.agents/handoffs/cursor-storage-gate.md`.

## One-line

CELIR refuses a frozen VCS object-store API; Phase B may path-dep documented building blocks behind `arks-store` only; Phase C may open only for an explicit clean-room subset that keeps Git-compat filesystem semantics authoritative and leaves power-loss, multi-writer (arks-lease), and learning Refuse outside the claim.
