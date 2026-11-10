---
id: evidence:root/20261109030456
title: M12 Cm-1 clean-room G17 concurrency issue-compare
status: active
created: 2026-11-09T03:04:56Z
updated: 2026-11-10T00:57:42Z
tags: [g17, m12, cm-1, clean-room, concurrency, issue-resolution, verified]
rel:
  depends_on: [plan:root/20261109024634, sot:root/program-roadmap, sot:root/interop-test-matrix, sot:root/multi-agent-git-concurrency-scenarios, sot:root/product-verification-clean-room, sot:root/production-residual-register, evidence:root/20261109114207, research:root/20261109023117]
  evidence: []
files:
  - {path: receipt.json, sha256: fb134ae780e6309870985a2edf4ffebba5c83501bc6dbb138f3b4d7f041b5dc8, bytes: 1220, store: repo}
---

# M12 Cm-1 — clean-room G17 concurrency issue-compare

## Context

Closes Wave **Cm-1** of [[plan:root/20261109024634]] under the Issue-resolution Done rule ([[sot:root/program-roadmap]] v0.5.0): each claimed ID reproduced in Docker — stock **`git`** exhibits pathology; **`arks`** meets resolved criteria; comparative receipt attached. Does **not** redefine M9/M10/M11 exits. Does **not** start M14 / G19 perf work. Unlisted catalog IDs remain Open.

Attached: `receipt.json` (`arks-g17-issue-compare/v1`, wave `Cm-1`).

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` services `g17` (+ regression `g14`, `g16`) |
| Entrypoints | `verify/scripts/run-g17.sh`, `run-g14.sh`, `run-g16.sh` |
| Image | `arks-verify-m1:0.1.0` |
| Image digest | `sha256:bab37bd1ea03832a5ca30aa41e598eafc8bef7a5b1026d65acefb75e3f4a73c1` |
| Stock Git (in-image) | `git version 2.39.5` |
| rustc (in-image) | `rustc 1.99.1 (58cf8f9c3 2026-10-13)` |
| `arks` profile (G17) | **release** |
| Tree SHA at G17 run | `3c4c54734d89e9c704732cca9c5391168c1e2705` (pre-land working tree) |
| Tree SHA (evidence tip) | `0c82325` (land commit; pin below if amended) |
| Host Docker engine | Linux (`desktop-linux`) |
| `CARGO_BUILD_JOBS` | `2` |

## Cm-1 G17 results

| ID | git | arks | Notes |
|----|-----|------|-------|
| AGENT-006 / IDX-008 | pathology_pass | resolved_pass | Held `index.lock` blocks git add; arks status `--no-optional-locks` / `ARKS_PROFILE=agent` creates no lock; writer proceeds |
| LOCK-002 | pathology_pass | resolved_pass | Stale lock blocks git; `arks lock inspect` + `clear --stale`; live PID refuse; write path never skips locks |
| WT-001 | pathology_pass | resolved_pass | Common-dir ref lock fails git/arks update-ref; locked worktree remove fail-closed |
| SUB-001 | pathology_pass | resolved_pass | Submodule `index.lock` fails writers; super status separate domain; DotGitFile worktree honesty |

## Regression

| Gate | Outcome |
|------|---------|
| G14 | PASS (embedded G13…G1 + gix-exit) |
| G16 functional | PASS (`ARKS_JOURNEY_ITERATIONS=1` advisory timing; no ≥20% bar) |
| gix-exit | PASS |
| `spec check` | See land commit |

## Mitigations landed (Cm-1 only)

- `arks status --no-optional-locks` + `GIT_OPTIONAL_LOCKS=0` / `ARKS_PROFILE=agent` (status never takes optional locks).
- `arks lock inspect` / `arks lock clear --stale` [`--force-stale`] — lease ≠ skip Git locks.
- Submodule / `.git` file discovery returns checkout worktree (lock-domain honesty).

## Non-claims

- Does **not** redefine M9/M10/M11.
- No L3; no CELIR; G8 deferred.
- No ≥20% / M14 / G19.
- Unlisted catalog IDs (IDX-001, HOOK-*, PACK-004, AGENT-003/005/007, …) remain **Open**.

## Reproduce

```bash
docker compose -f verify/docker-compose.yml run --rm g17
docker compose -f verify/docker-compose.yml run --rm g14
docker compose -f verify/docker-compose.yml run --rm -e ARKS_JOURNEY_ITERATIONS=1 g16
spec check
```
