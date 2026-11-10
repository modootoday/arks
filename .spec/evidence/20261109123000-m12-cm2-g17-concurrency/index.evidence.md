---
id: evidence:root/20261109123000
title: M12 Cm-2 clean-room G17 concurrency issue-compare
status: active
created: 2026-11-09T12:30:00Z
updated: 2026-11-09T22:48:32Z
tags: [g17, m12, cm-2, clean-room, concurrency, issue-resolution, verified]
rel:
  depends_on: [plan:root/20261109024634, sot:root/program-roadmap, sot:root/interop-test-matrix, sot:root/multi-agent-git-concurrency-scenarios, sot:root/product-verification-clean-room, evidence:root/20261109030456, research:root/20261109023117]
  evidence: []
files:
  - {path: receipt.json, sha256: 92c2cf6fa10d00536eedcdf7b4190da419a195f1e1e4ac905b92a72aa23d7344, bytes: 2143, store: repo}
---

# M12 Cm-2 — clean-room G17 concurrency issue-compare

## Context

Closes Wave **Cm-2** of [[plan:root/20261109024634]] under the Issue-resolution Done rule ([[sot:root/program-roadmap]] v0.5.0): each claimed ID reproduced in Docker — stock **`git`** exhibits pathology; **`arks`** meets resolved criteria; comparative receipt attached. Cm-1 rows re-verified green in the same receipt. Does **not** redefine M9/M10/M11 exits. Does **not** start M14 / G19 perf work. Unlisted catalog IDs remain Open.

Attached: `receipt.json` (`arks-g17-issue-compare/v1`, wave `Cm-2`).

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` services `g17` (+ regression `g14`, `g16`) |
| Entrypoints | `verify/scripts/run-g17.sh`, `run-g14.sh`, `run-g16.sh` |
| Image | `arks-verify-m1:0.1.0` |
| Image digest | `sha256:bab37bd1ea03832a5ca30aa41e598eafc8bef7a5b1026d65acefb75e3f4a73c1` |
| Stock Git (in-image) | `git version 2.39.5` |
| Tree SHA at G17 run | `03ba33772dba8073b6bb58388f3a88fd7153b8e1` (structural tip before Cm-2 land) |
| Tree SHA (evidence tip) | `8014f89` |
| Host Docker engine | Linux (`desktop-linux`) |

## Cm-2 G17 results

| ID | git | arks | Notes |
|----|-----|------|-------|
| IDX-001 | pathology_pass | resolved_pass | Exclusive `index.lock` fails git/arks writers; arks message includes retry/exclusive honesty |
| HOOK-001 / HOOK-006 | pathology_pass | resolved_pass | Refuse inventing async hook runners; `--no-verify` escape; sync-critical honesty |
| PACK-004 / MAINT-003 | pathology_pass | resolved_pass | `arks gc`/`repack`/`maintenance` refuse-closed under writer/prune race class |
| AGENT-003 | pathology_pass | resolved_pass | Lock-fail + `arks lock inspect` teach backoff/jitter; no M12 lease scheduler |

## Cm-1 regression (same receipt)

| ID | git | arks |
|----|-----|------|
| AGENT-006 / IDX-008 | pathology_pass | resolved_pass |
| LOCK-002 | pathology_pass | resolved_pass |
| WT-001 | pathology_pass | resolved_pass |
| SUB-001 | pathology_pass | resolved_pass |

## Regression

| Gate | Outcome |
|------|---------|
| G14 | PASS |
| G16 functional | PASS (`ARKS_JOURNEY_ITERATIONS=1`; no ≥20% bar) |
| gix-exit | PASS |

## Mitigations landed (Cm-2)

- IDX-001: exclusive lock fail-closed messages with retry guidance (index/refs); no sharded writer.
- HOOK-001/006: detect hooks; refuse inventing async runners; `--no-verify` claimed escape.
- PACK-004/MAINT-003: refuse-closed `gc`/`repack`/`maintenance`.
- AGENT-003: backoff/retry guidance on `LockFailed` + `arks lock inspect` note.

## Non-claims

- Does **not** redefine M9/M10/M11.
- No L3; no CELIR; G8 deferred.
- No ≥20% / M14 / G19.
- Unlisted IDs (IDX-002, AGENT-005/007, CORR-001, FS-006, …) remain **Open** for Cm-3 / M13.

## Reproduce

```bash
docker compose -f verify/docker-compose.yml run --rm g17
docker compose -f verify/docker-compose.yml run --rm g14
docker compose -f verify/docker-compose.yml run --rm -e ARKS_JOURNEY_ITERATIONS=1 g16
spec check
```
