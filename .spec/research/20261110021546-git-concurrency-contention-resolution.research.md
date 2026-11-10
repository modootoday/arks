---
id: research:root/20261110021546
title: Full Git contention resolution — deferred promotion map
status: active
created: 2026-11-10T02:15:46Z
updated: 2026-11-10T02:15:46Z
tags: [git, concurrency, multi-agent, contention, g017-concurrency-issue-compare, g018-concurrency-issue-compare, deferred, promote, cx-3, m20]
rel:
  depends_on:
    - research:root/20261109023117
    - sot:root/multi-agent-git-concurrency-scenarios
    - sot:root/lease-concurrency-intent
    - sot:root/program-roadmap
    - sot:root/git-compat-surface
    - adr:root/20261108232800
    - plan:root/20261110010000
---

# Full Git contention resolution — deferred promotion map

## Question

After M12/M13/M19 (Cx-1/Cx-2), **170** catalog IDs remain **Closed-deferred**. Which of those are **promoteable** into new G17 (mitigate) / G18 (differentiate) claim waves for honest full contention-resolution progress, which stay **policy-gated**, and which are **refuse / out-of-product-scope** — citing **primary** git-scm / Git project docs only?

## Method

1. Re-read baseline taxonomy [[research:root/20261109023117]] + disposition register in [[sot:root/multi-agent-git-concurrency-scenarios]].
2. Cross-check hot spots against primary docs (summarize + URL; do not paste large copyrighted text).
3. Classify each Closed-deferred ID into a promotion bucket with induce-compare Done rules.
4. Recommend ordered claim waves (Cx-3+, Cd-4+) without inventing wire/object semantics or reopening L3.

Research snapshot date: **2026-11-10**. Baseline tip at research start: `6db356f` (M19 Cx-2 closed).

## Primary source refresh (cite, do not copy)

| Topic | URL | Scope used |
|-------|-----|------------|
| Lockfile API | https://git-scm.com/docs/api-lockfile | Exclusive `O_CREAT\|O_EXCL`, rename commit, writers-only exclusion, `LOCK_DIE_ON_ERROR`, same-FS rename |
| Repository layout | https://git-scm.com/docs/gitrepository-layout | `packed-refs`, `shallow`, `modules`, `worktrees`, `info/sparse-checkout`, format v1 `extensions.*` hard refuse |
| pack-refs | https://git-scm.com/docs/git-pack-refs | Single `packed-refs` rewrite; `--all` packs loose tips |
| update-ref | https://git-scm.com/docs/git-update-ref | CAS, transactions, reflog append |
| worktree | https://git-scm.com/docs/git-worktree | lock/prune/move; submodules block move |
| sparse-checkout | https://git-scm.com/docs/git-sparse-checkout | cone file + index skip-worktree |
| notes | https://git-scm.com/docs/git-notes | `refs/notes/*` updates; notes-merge worktree state |
| gc | https://git-scm.com/docs/git-gc | auto, `--force`, prune NOTES, `gc.log` suppression class |
| githooks | https://git-scm.com/docs/githooks | pre-auto-gc, post-index-change, reference-transaction |
| status optional locks | https://git-scm.com/docs/git-status | `--no-optional-locks` |
| reftable | https://git-scm.com/docs/reftable | alternate ref storage contention class |
| clone | https://git-scm.com/docs/git-clone | destination path collision |
| submodule | https://git-scm.com/docs/gitsubmodules | nested repos under `modules/` |
| packed-refs lock lore | http://public-inbox.org/git/xmqq38337zy1.fsf@gitster.dls.corp.google.com/T/ | Why packed-refs needs timeout retries |

## Gap analysis vs real Git races

Prior catalog (221) already covers index/ref/worktree/pack/sequencer/hooks/network/fetch-push/submodule/TOCTOU/Windows/LFS/CI. Post-Cx-2 gaps are **not** missing class names — they are **unclaimed proof** plus a few deepenings:

| Gap class | Examples still deferred | Primary anchor | Promoteability |
|-----------|-------------------------|----------------|----------------|
| packed-refs global lock | REF-012, REF-022, REF-003 | pack-refs + lockfile + lore | **Cx mitigate** (lock honor + reader honesty) |
| Notes / reflog append | REF-016, REF-009, MISC-014, MISC-017 | notes + update-ref logging | **Cx mitigate** (lock/state refuse) |
| Index plumbing under lock | IDX-013, IDX-014, IDX-004/005/010/011 | update-index / read-tree / sparse-index | **Cx mitigate** for lock honor; split-index deepen later |
| Sparse cone rewrite | SPARSE-001, MISC-018, MISC-008 | sparse-checkout + layout `info/sparse-checkout` | **Cx mitigate** |
| Worktree admin races | WT-005/006/009/010, TOCTOU-007, SUB-008 | worktree + layout | **Cx mitigate** for move/prune/submodule block |
| Pack keep / MIDX / cruft | PACK-001/002/003/005/008/009, MAINT-005/006 | gc / pack docs | Mix: honesty notes now; full MIDX later |
| Shallow / promisor | SHAL-001/003/004, PACK-007, NET-010 | shallow file + partial clone | **Cx** for shallow.lock; network/promisor often G8/policy |
| Hook mid-critical-section | HOOK-002/003/005/007 | githooks | **Cx** honesty / fail-closed; async runners stay out |
| Config / format v1 | CFG-001..004 | layout format v1 extensions MUST NOT proceed | **Cx** for CFG-004; concurrent config merge is **Cd** |
| Clone dest collision | NET-009 | git-clone | **Cx mitigate** |
| FS/NFS/SMB/AV/WSL | FS-001..005, FS-008..013, LOCK-009, WIN-001/002 | ops + lockfile same-FS assumption | Mostly **guidance / host-specific**; limited Docker induce |
| Network stampede / auth | NET-001..005/007/008/011, CRED-* | fetch/push/credentials | **Policy G8** and/or **Cd** lease namespaces |
| LFS content lock mismatch | LFS-* | git-lfs (extension; cite issues only as symptoms) | Mitigate guidance; not core object store |
| Agent blackboard / lease uplift | AGENT-001/004/008..012, REF-002/010, IDX-003, SEQ-005, PERF-* | lease SoT + ADR L3 refuse | **Cd differentiate**; L3 stays refuse-closed |
| CI shared workspace | CI-001..006 | CI hygiene not Git wire | Mitigate docs; some G17 if local lock plant fits |
| Corruption ops anti-patterns | CORR-003..007 | gc NOTES + replace/alternates | Selective Cx refuse; hostile cases stay deferred |

No new invented lock dialect. No CELIR. L3 N-way pipeline remains refuse-closed ([[adr:root/20261108232800]]).

## Promotion buckets (Closed-deferred → disposition)

Counts at research time: Claimed **42** + N/A **9** + Closed-deferred **170** = **221**.

### A — Promoteable mitigate (G17 / Phase Cx) — next waves

Local filesystem races with clear induce → stock pathology → arks resolved oracle (fail-closed / inspect honesty). Prefer ~10–12 IDs per wave.

**Wave Cx-3 (recommended first):**
`REF-012`, `REF-016`, `IDX-014`, `SPARSE-001`, `MISC-018`, `SUB-008`, `TOCTOU-007`, `PACK-009`, `MAINT-002`, `NET-009`, `CFG-004`, `MISC-017`

**Wave Cx-4+ candidates (after Cx-3):**
`REF-009`, `REF-022`, `IDX-013`, `IDX-004`, `IDX-010`, `IDX-011`, `HOOK-002`, `HOOK-003`, `HOOK-005`, `SHAL-001`, `SHAL-003`, `MISC-002`, `MISC-003`, `MISC-011`, `MISC-014`, `MISC-019`, `MISC-020`, `OBJ-001`, `OBJ-005`, `OBJ-008`, `PACK-001`, `PACK-008`, `WT-005`, `WT-006`, `WT-009`, `WT-010`, `SUB-003`, `SUB-006`, `SUB-009`, `SEQ-004`, `FSM-001`, `FSM-002`, `FSM-004`, `CORR-004`, `CORR-005`, `LOCK-003`, `LOCK-007`, `FS-008`, `FS-010`, `CI-001`, `CI-005`

### B — Promoteable differentiate (G18 / Phase Cd)

Needs lease/grant/namespace/backoff product shapes; dual-path honesty; L3 still refuse-closed.
`AGENT-001`, `AGENT-004`, `AGENT-008`, `AGENT-009`, `AGENT-010`, `AGENT-011`, `AGENT-012`, `REF-002`, `REF-004`, `REF-005`, `REF-008`, `REF-010`, `REF-014`, `REF-015`, `REF-019`, `REF-023`, `IDX-003`, `IDX-007`, `IDX-009`, `IDX-012`, `SEQ-005`, `NET-001`, `NET-002`, `NET-007`, `PERF-001`, `PERF-002`, `PERF-005`, `PERF-006`, `PERF-007`, `PERF-008`, `PERF-009`, `PERF-010`, `OBJ-002`, `OBJ-003`, `PACK-007`, `SHAL-002`, `SUB-002`, `SUB-004`, `SUB-007`, `SUB-010`, `WT-003`, `WT-007`, `WT-008`, `MAINT-001`, `HOOK-004`, `CFG-001`, `CFG-003`, `MISC-004`, `MISC-010`, `MISC-015`, `MISC-016`, `TOCTOU-001`, `TOCTOU-002`, `TOCTOU-006`, `CI-003`, `CI-004`, `LFS-002`

### C — Policy / remote-gated (stay Closed-deferred until G8 unlock)

Live forge / secrets / credential helpers / rate-limit hosts:
`CRED-001`, `CRED-002`, `NET-003`, `NET-004`, `NET-005`, `NET-008`, `NET-011`, `TOCTOU-004`, `LFS-001`, `LFS-003`, `LFS-004`, `LFS-005`, `LFS-006`, `HOOK-007`, `CI-002`, `CI-006`, `G8/G9` residuals (not catalog rows)

### D — Host-FS / platform limited (Closed-deferred; guidance-only unless dedicated harness)

`FS-001`, `FS-002`, `FS-003`, `FS-004`, `FS-005`, `FS-009`, `FS-011`, `FS-012`, `FS-013`, `WIN-001`, `WIN-002`, `LOCK-009`, `REF-020`, `AGENT-004` (NFS cross-host also in B)

### E — Refuse / out-of-product (do not green-claim)

- **L3 HighConcurrency valve** — refuse-closed (ADR); not a catalog row green.
- Opportunity **out-of-scope** already N/A×9 (`FS-007`, `FS-014`, `HOOK-008`, `MAINT-004`, `MISC-006`, `NET-006`, `OBJ-004`, `REF-017`, `WIN-003`).
- Absolute PERF SLOs remain **M14/G19-separated** (issue track must not claim ms budgets).

## Honesty bar for “full resolution”

**Superseded for program exit by M21 Deferred=0** ([[plan:root/20261110140000]], [[sot:root/multi-agent-git-concurrency-scenarios]] v disposition register):

Full catalog closure is reached only when **Deferred count is 0** and every ID has exactly one **terminal** disposition:

1. **Claimed+Docker clean-room closed** (G17/G18), OR
2. **Refuse-closed** with ADR+SoT (**L3-class only**), OR
3. **N/A-out-of-scope** with primary-doc cite.

**Closed-deferred / policy-deferred / host-FS-limited / Docker-induce-thin are not terminal success dispositions** — they are M21 work-queue unlock items. Bucket A/B exhaustion alone is **not** Done.

Until Deferred=0: **do not claim “all Git races fixed.”** Dispose ≠ solved; Deferred≠0 ≠ closed.

## Recommendations (actionable)

1. Canonicalize **M20 Phase Cx-3** claim-set (12 IDs above) with induce-compare Done rules; extend G17 harness (do not invent G22).
2. After each Cx wave Done: immediately promote next Cx bucket from A; then schedule **Cd-4** from B for lease uplift.
3. Keep CELIR excluded; keep L3 refuse-closed unless superseding ADR.
4. Sync catalog disposition counts after every wave close.

## Open decisions

1. Whether Cd-4 runs interleaved with Cx-4 or only after mitigate local races exhaust bucket A.
2. Whether host-FS (bucket D) ever graduates to a dedicated Docker/Windows clean-room matrix or stays guidance forever.
3. G8 policy unlock timeline for bucket C (not this research’s exit).

## Links

- Catalog SoT: [[sot:root/multi-agent-git-concurrency-scenarios]]
- Baseline research: [[research:root/20261109023117]]
- M19 Cx plan: [[plan:root/20261110010000]]
- Roadmap: [[sot:root/program-roadmap]]
