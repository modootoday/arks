---
id: evidence:root/20261109194858
title: M23 L3 HighConcurrency g11l3 dual-path unlock
status: active
created: 2026-11-09T19:48:58Z
updated: 2026-11-09T19:50:32Z
tags: [m23, l3, high-concurrency, g11l3, clean-room, dual-path, verified]
rel:
  depends_on: [adr:root/20261109194200, plan:root/20261109194208, sot:root/arks-lease, sot:root/lease-concurrency-intent, sot:root/product-verification-clean-room, sot:root/program-roadmap, sot:root/production-residual-register, evidence:root/20261108231249]
  evidence: []
files:
  - {path: receipt.json, sha256: 3fc1b5fb003426aaad7f154355ad96c9d17b70628c85556b745ca9d43a6193be, bytes: 398, store: repo}
---

# M23 — L3 HighConcurrency g11l3 dual-path unlock

## Context

Unlocks `WriterMode::HighConcurrency` for the **claimed shape** locked by [[adr:root/20261109194200]] after Docker clean-room **G11-L3** PASS. Does **not** claim shared-index N-way. Does **not** rewrite M10 L1→L2 [[evidence:root/20261108231249]].

## Clean-room receipt

| Field | Value |
|-------|-------|
| Gate | **G11-L3** (`verify/scripts/run-g11l3.sh` / compose `g11l3`) |
| Matrix | `verify/matrices/matrix-l3.md` |
| Stock git (image) | `git version 2.39.5` |
| rustc (image) | `1.99.1` |
| Result | **PASS** |
| Suite | `cargo test -p arks-lease` (10 tests incl. N=32 + N=128 shard storm + lock-additive) + `verify/gates/g011-lease-product` regress |
| gix / celir | zero edges (metadata scan) |
| Tip at unlock | Hc-3 unlock commit on `origin/main` (after `dd2712b`); post-unlock Docker g11l3 revalidated PASS |

## Dual-path honesty

| Path | Proven |
|------|--------|
| **GitCompat** | Default `WriterSession`; L1/L2 + lease ≠ skip Git locks (`lease_held_still_honors_git_ref_lock`, competing lock → Pipeline) |
| **HighConcurrency** | Opt-in mode accepted after unlock; pipeline parallel prep + ordered batch apply + tip verify on distinct ref shards |
| **Export/import** | Identity — claimed apply writes stock-readable loose objects + refs under `.git/` (no divergent private tip encoding) |
| **Shared-index N-way** | **OUT** — not claimed |

## Disposition

**L3 claimed shape = Claimed** for lease+batch-apply+worktree/ref-shard. CELIR excluded. No fake greens. No “faster than Git” marketing.
