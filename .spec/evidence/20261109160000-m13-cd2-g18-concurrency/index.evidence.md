---
id: evidence:root/20261109160000
title: M13 Cd-2 clean-room G18 concurrency issue-compare
status: active
created: 2026-11-09T16:00:00Z
updated: 2026-11-10T00:57:43Z
tags: [g018-concurrency-issue-compare, m13, cd-2, clean-room, concurrency, differentiate-beyond-git, lease, fencing, worktree, submodule, issue-resolution, verified]
rel:
  depends_on: [plan:root/20261109024635, adr:root/20261109032955, sot:root/program-roadmap, sot:root/interop-test-matrix, sot:root/multi-agent-git-concurrency-scenarios, sot:root/lease-concurrency-intent, sot:root/product-verification-clean-room, evidence:root/20261109153000, research:root/20261109023117]
  evidence: []
files:
  - {path: receipt.json, sha256: f6890a1865250fb1c8a8667a655516a08d74a120ef773c1e9810e94dcb923afa, bytes: 1967, store: repo}
---

# M13 Cd-2 — clean-room G18 concurrency issue-compare

## Context

Closes Wave **Cd-2** of [[plan:root/20261109024635]]: LOCK-002 TTL fencing + WT-001/SUB-001/AGENT-007 cross-domain leases. Cd-1 rows re-verified green in the same receipt. Grant schema amend: [[adr:root/20261109032955]]. Does **not** redefine M9/M10/M11. Does **not** start M14/G19.

Attached: `receipt.json` (`arks-g018-concurrency-issue-compare-issue-compare/v1`, wave `Cd-2`).

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` service `g018-concurrency-issue-compare` |
| Image | `arks-verify-m1:0.1.0` |
| Image digest | `sha256:bab37bd1ea03832a5ca30aa41e598eafc8bef7a5b1026d65acefb75e3f4a73c1` |
| Stock Git | `git version 2.39.5` |
| Tree SHA at G18 run | `29ac2a8aa2357cb3928f4c81e822721bb09f784b` (Cd-1 tip)
| Tree SHA (evidence tip) | `5994ba5` |

## Cd-2 G18 results

| ID | git | arks | Notes |
|----|-----|------|-------|
| LOCK-002 | pathology_pass | resolved_pass | TTL fencing via `lease expire`; Git locks honored |
| WT-001 | pathology_pass | resolved_pass | `domain:worktree:` lease; locks honored |
| SUB-001 | pathology_pass | resolved_pass | `domain:submodule:` lease; locks honored |
| AGENT-007 | pathology_pass | resolved_pass | Composed domain leases; inspect + overlap AlreadyHeld |

## Cd-1 regression (same receipt)

REF-001, REF-006, REF-011, AGENT-003 — all pathology_pass / resolved_pass.

## Non-claims

No L3; no CELIR; no ≥20%/M14; unlisted differentiate IDs Open for Cd-3+.

## Reproduce

```bash
docker compose -f verify/docker-compose.yml run --rm g018-concurrency-issue-compare
spec check
```
