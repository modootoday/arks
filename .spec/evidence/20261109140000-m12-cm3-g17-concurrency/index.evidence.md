---
id: evidence:root/20261109140000
title: M12 Cm-3 clean-room G17 concurrency issue-compare
status: active
created: 2026-11-09T14:00:00Z
updated: 2026-11-09T22:48:33Z
tags: [g17, m12, cm-3, clean-room, concurrency, issue-resolution, verified]
rel:
  depends_on: [plan:root/20261109024634, sot:root/program-roadmap, sot:root/interop-test-matrix, sot:root/multi-agent-git-concurrency-scenarios, sot:root/product-verification-clean-room, evidence:root/20261109030456, evidence:root/20261109123000, research:root/20261109023117]
  evidence: []
files:
  - {path: receipt.json, sha256: 8493071378fc9caaf03dada2b3973c71106e4bc8910977aafb07448f8ec77c55, bytes: 3245, store: repo}
---

# M12 Cm-3 — clean-room G17 concurrency issue-compare

## Context

Closes Wave **Cm-3** of [[plan:root/20261109024634]] under the Issue-resolution Done rule ([[sot:root/program-roadmap]] v0.5.0): each claimed ID reproduced in Docker — stock **`git`** exhibits pathology; **`arks`** meets resolved criteria; comparative receipt attached. Cm-1 and Cm-2 rows re-verified green in the same receipt. Does **not** redefine M9/M10/M11 exits. Does **not** start M14 / G19 perf work. Unlisted catalog IDs remain Open.

Attached: `receipt.json` (`arks-g17-issue-compare/v1`, wave `Cm-3`).

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` services `g17` (+ regression `g14`, `g16`) |
| Entrypoints | `verify/scripts/run-g17.sh`, `run-g14.sh`, `run-g16.sh` |
| Image | `arks-verify-m1:0.1.0` |
| Image digest | `sha256:bab37bd1ea03832a5ca30aa41e598eafc8bef7a5b1026d65acefb75e3f4a73c1` |
| Stock Git (in-image) | `git version 2.39.5` |
| Tree SHA at G17 run | `c97a0a17161e4189e5be2661f762ea8e1fad412e` (Cm-2 tip before Cm-3 land) |
| Tree SHA (evidence tip) | `ee34b29` |
| Host Docker engine | Linux (`desktop-linux`) |

## Cm-3 G17 results

| ID | git | arks | Notes |
|----|-----|------|-------|
| IDX-002 | pathology_pass | resolved_pass | Background/optional status lock class; default arks status takes no optional locks |
| AGENT-005 | pathology_pass | resolved_pass | Evicted/dead PID stale lock inspect+clear; write path honors locks |
| AGENT-007 | pathology_pass | resolved_pass | Worktree+submodule multi-domain fail-closed; AGENT-007 inspect honesty |
| CORR-001 | pathology_pass | resolved_pass | Refuse gc/repack/maintenance incl. prune=now under writers |
| FS-006 | pathology_pass | resolved_pass | Bind-mount dual-writer honesty; inspect FS-006; honor locks; clear only dead |

## Cm-1 / Cm-2 regression (same receipt)

| ID | git | arks |
|----|-----|------|
| AGENT-006 / IDX-008 | pathology_pass | resolved_pass |
| LOCK-002 | pathology_pass | resolved_pass |
| WT-001 | pathology_pass | resolved_pass |
| SUB-001 | pathology_pass | resolved_pass |
| IDX-001 | pathology_pass | resolved_pass |
| HOOK-001 / HOOK-006 | pathology_pass | resolved_pass |
| PACK-004 / MAINT-003 | pathology_pass | resolved_pass |
| AGENT-003 | pathology_pass | resolved_pass |

## Regression

| Gate | Outcome |
|------|---------|
| G14 | PASS |
| G16 functional | PASS (`ARKS_JOURNEY_ITERATIONS=1`; no ≥20% bar) |
| gix-exit | PASS |

## Mitigations landed (Cm-3)

- IDX-002: default status never takes optional locks (background status vs writer class).
- AGENT-005: `arks lock inspect` / `clear --stale` eviction/dead-PID honesty; lease ≠ skip live locks.
- AGENT-007: worktree+submodule composition fail-closed; multi-domain inspect note.
- CORR-001: refuse-closed `gc`/`repack`/`maintenance` citing prune=now corruption class.
- FS-006: inspect warns bind-mount / cross-ns dual-writer; write path honors locks.

## Non-claims

- Does **not** redefine M9/M10/M11.
- No L3; no CELIR; G8 deferred.
- No ≥20% / M14 / G19.
- Unlisted IDs remain **Open** for Cm-4 / M13.

## Reproduce

```bash
docker compose -f verify/docker-compose.yml run --rm g17
docker compose -f verify/docker-compose.yml run --rm g14
docker compose -f verify/docker-compose.yml run --rm -e ARKS_JOURNEY_ITERATIONS=1 g16
spec check
```
