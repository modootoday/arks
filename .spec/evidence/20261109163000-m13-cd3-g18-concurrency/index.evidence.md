---
id: evidence:root/20261109163000
title: M13 Cd-3 clean-room G18 concurrency issue-compare
status: active
created: 2026-11-09T16:30:00Z
updated: 2026-11-09T22:48:33Z
tags: [g18, m13, cd-3, clean-room, concurrency, differentiate-beyond-git, lease, hooks, maint, issue-resolution, verified]
rel:
  depends_on: [plan:root/20261109024635, adr:root/20261109032955, sot:root/program-roadmap, sot:root/interop-test-matrix, evidence:root/20261109153000, evidence:root/20261109160000, research:root/20261109023117]
  evidence: []
files:
  - {path: receipt.json, sha256: 94391abcd45411a1962d18a7efb4dfa8f04bbd384c79b9f13a097dc72156f0bd, bytes: 2838, store: repo}
---

# M13 Cd-3 — clean-room G18 concurrency issue-compare

## Context

Closes Wave **Cd-3** of [[plan:root/20261109024635]]: HOOK budgets, maint/gc leases, AGENT-002 merge namespace, IDX-001 path-set. Cd-1/Cd-2 regression green in the same receipt. Together with Cd-1/Cd-2 this completes the **M13 claimed set** (explicit wave IDs). Residual catalog rows (NET fetch queues, AGENT-001/SEQ blackboard, PERF batch beyond lease wait, L3) remain **Open / unclaimed** — not required for this M13 exit. Does **not** redefine M9/M10/M11. Does **not** start M14 until this evidence lands and plan exit is recorded.

Attached: `receipt.json` (`arks-g18-issue-compare/v1`, wave `Cd-3`).

## Environment

| Field | Value |
|-------|-------|
| Compose | `g18` |
| Image | `arks-verify-m1:0.1.0` / `sha256:bab37bd1ea03832a5ca30aa41e598eafc8bef7a5b1026d65acefb75e3f4a73c1` |
| Stock Git | `git version 2.39.5` |
| Tree SHA at G18 run | `68d0deee7cd73081e73e3dc484be898c09a7e4ff`
| Tree SHA (evidence tip) | `2be05ad` |

## Cd-3 results

| ID | git | arks |
|----|-----|------|
| HOOK-001/HOOK-006 | pathology_pass | resolved_pass |
| PACK-004/MAINT-003 | pathology_pass | resolved_pass |
| AGENT-002 | pathology_pass | resolved_pass |
| IDX-001 | pathology_pass | resolved_pass |

## Regression

Cd-1 (REF-001/006/011, AGENT-003) + Cd-2 (LOCK-002, WT-001, SUB-001, AGENT-007) — all dual PASS.

## Non-claims

No L3; no CELIR; no ≥20%/M14; unclaimed catalog IDs remain Open.

## Reproduce

```bash
docker compose -f verify/docker-compose.yml run --rm g18
spec check
```
