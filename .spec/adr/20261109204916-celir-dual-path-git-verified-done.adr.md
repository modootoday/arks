---
id: adr:root/20261109204916
title: CELIR dual-path Git verified Done (not ObjectStore)
status: active
created: 2026-11-09T20:49:16Z
updated: 2026-11-10T05:49:00Z
tags: [celir, indexing-service, dual-path, verified, clean-room, honesty, object-store-refuse]
rel:
  depends_on:
    - sot:root/storage-celir-boundary
    - adr:root/20261110052030
    - sot:root/celir-index-dual-path
    - sot:root/product-verification-clean-room
    - sot:root/celir-dual-path-git-verified
---

# CELIR dual-path Git verified Done (not ObjectStore)

## Context

G22 closed **index-contention verified** ([[evidence:root/20261109204428]]) and explicitly **refused** the marketing phrase “CELIR-backed Git verified.” That refusal was honest: CELIR is not an ObjectStore. Residual honesty still left three non-passes open:

1. No clean-room receipt proving Git porcelain/interop **while** `celir-index` build/publish/open/query works on the same repo.
2. G22 scale stopped at N=32/128 (not higher hundreds under sole-runner OOM policy).
3. G22 was separate from G11-L3; no joint HighConcurrency + CELIR index clean-room gate.

Ambiguous “CELIR-backed Git” language invites ObjectStore confusion. Product needs an **honest Done** name and gate.

## Decision

1. **Accepted vocabulary:** Prefer **“CELIR dual-path Git verified”** (or “dual-path Git + CELIR index verified”). Treat bare “CELIR-backed Git verified” as **forbidden** unless the claim body explicitly states FS `.git` sole ObjectStore + CELIR index sidecar only.
2. **Honest Done (G23):** Docker clean-room proves, on one worktree:
   - FS `.git` is the sole ObjectStore (`put`/`get`/`exists` via `FilesystemGitStore` only).
   - Git porcelain/interop subset (read/write object + commit-class tip + status-class stock readability, or G14-class subset) succeeds.
   - `celir-index` build/publish/open/query (rebuild-from-`.git` + lookup) succeeds on the sidecar.
   - In-image stock Git can still open/read the repo (`cat-file` / `show-ref` / equivalent).
3. **Still Refuse:** CELIR as ObjectStore; put/get/exists via CELIR; default-on `celir-index`; lease ≠ skip Git locks; host-only greens as Verified.
4. **Scale Done (G25):** Additive to G22 — N≥256 required for Verified; higher tiers (384/512) under sole-runner OOM policy (PASS or documented hard-block; no fake green).
5. **Joint Done (G24):** HighConcurrency claimed-shape path + CELIR index together in clean-room; shared-index N-way remains OUT; does not redefine G11-L3 as CELIR-bearing.

## Consequences

- New gates **G23** / **G24** / **G25** and owning SoTs become the residual close path after G22.
- Residual / roadmap / public-surface MAY say **CELIR dual-path Git verified** only with a G23 PASS receipt.
- ObjectStore Refuse and [[adr:root/20261110052030]] remain durable.
