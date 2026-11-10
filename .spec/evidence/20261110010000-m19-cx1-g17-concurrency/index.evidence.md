---
id: evidence:root/20261110010000
title: M19 Cx-1 clean-room G17 concurrency claim expansion
status: active
created: 2026-11-10T01:00:00Z
updated: 2026-11-10T01:00:00Z
tags: [g17, m19, cx-1, clean-room, concurrency, issue-resolution, verified]
rel:
  depends_on: [plan:root/20261110010000, sot:root/program-roadmap, sot:root/interop-test-matrix, sot:root/multi-agent-git-concurrency-scenarios, sot:root/product-verification-clean-room, sot:root/production-residual-register, evidence:root/20261109140000, research:root/20261109023117]
  evidence: []
files:
  - {path: receipt.json, sha256: 6d3048a5bb425a67f1a2daadf02aaf58993af062b021fabfd8a6bde6ad53f602, bytes: 5107, store: repo}
---

# M19 Cx-1 — clean-room G17 concurrency claim expansion

## Context

Closes Wave **Cx-1** of [[plan:root/20261110010000]] under the Issue-resolution Done rule ([[sot:root/program-roadmap]] v0.9.0): each newly claimed ID reproduced in Docker — stock **`git`** exhibits pathology; **`arks`** meets resolved criteria; comparative receipt attached. M12 Cm-1..Cm-3 rows re-verified green in the same receipt (**23**/23 dual PASS). Does **not** redefine M9–M18 exits. Does **not** claim the ~221 catalog is solved. L3 remains refuse-closed; G8/G9 stay Closed-deferred.

Attached: `receipt.json` (`arks-g17-issue-compare/v1`, wave `Cx-1`).

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` service `g17` |
| Entrypoint | `verify/scripts/run-g17.sh` |
| Image | `arks-verify-m1:0.1.0` |
| Image digest | `sha256:5736c5687d3b5775e187ca3d21fbc5b2a5a2bf8965c5381778ee478eb185adf0` |
| Stock Git (in-image) | `git version 2.39.5` |
| Tree SHA at G17 run | `c8c14448b1a0292202cc08c8882e548cfed8df90` (pre-land tip) |
| Tree SHA (evidence tip) | `15cf0df` |
| Host Docker engine | Linux (`desktop-linux`) |

## Cx-1 G17 results (10 new)

| ID | git | arks | Notes |
|----|-----|------|-------|
| REF-007 | pathology_pass | resolved_pass | HEAD.lock fail-closed; inspect REF-007 |
| LOCK-001 | pathology_pass | resolved_pass | Exclusive O_CREAT lock fail-closed; inspect LOCK-001 |
| WT-002 | pathology_pass | resolved_pass | Same branch two worktrees fail-closed |
| WT-004 | pathology_pass | resolved_pass | Locked worktree remove refuse; unlock then remove |
| SEQ-001 | pathology_pass | resolved_pass | rebase-merge → refuse concurrent rebase |
| SEQ-002 | pathology_pass | resolved_pass | MERGE_HEAD → refuse rebase (git merge not-concluded class) |
| SEQ-003 | pathology_pass | resolved_pass | CHERRY_PICK_HEAD → refuse concurrent cherry-pick |
| REF-018 | pathology_pass | resolved_pass | MERGE_HEAD pseudoref → refuse conflicting porcelain |
| MISC-012 | pathology_pass | resolved_pass | update-index --refresh honors index.lock |
| TOCTOU-005 | pathology_pass | resolved_pass | No poll-then-skip; inspect TOCTOU-005 |

## Cm-1..Cm-3 regression (same receipt)

13 prior G17 rows: all `pathology_pass` / `resolved_pass` (AGENT-006/IDX-008 … FS-006).

## Aggregate

| Metric | Value |
|--------|------:|
| Scenario rows | 23 |
| git pathology_pass | 23 |
| arks resolved_pass | 23 |
| fail outcomes | 0 |

## Mitigations landed (Cx-1)

- Sequencer/pseudoref exclusivity (SEQ-001/002/003, REF-018) on rebase/cherry-pick entry.
- `update-index --refresh` honors held `index.lock` (MISC-012).
- Lock inspect notes for REF-007 / LOCK-001 / TOCTOU-005; index lock error cites LOCK-001.
- WT-002 / WT-004 proven via existing worktree fail-closed paths.

## Non-claims

- Does **not** mean ~221 catalog races are solved.
- Does **not** reopen/rewrite M12/M13 historical exits.
- No L3; no CELIR; G8 deferred; no ≥20% remapping.

## Reproduce

```bash
docker compose -f verify/docker-compose.yml run --rm g17
spec check
```
