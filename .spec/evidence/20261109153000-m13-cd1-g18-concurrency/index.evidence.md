---
id: evidence:root/20261109153000
title: M13 Cd-1 clean-room G18 concurrency issue-compare
status: active
created: 2026-11-09T15:30:00Z
updated: 2026-11-10T00:57:43Z
tags: [g018-concurrency-issue-compare, m13, cd-1, clean-room, concurrency, differentiate-beyond-git, lease, issue-resolution, verified]
rel:
  depends_on: [plan:root/20261109024635, adr:root/20261109032955, sot:root/program-roadmap, sot:root/interop-test-matrix, sot:root/multi-agent-git-concurrency-scenarios, sot:root/lease-concurrency-intent, sot:root/arks-lease, sot:root/product-verification-clean-room, evidence:root/20261108231249, evidence:root/20261109140000, research:root/20261109023117]
  evidence: []
files:
  - {path: receipt.json, sha256: 30978117e9039dafe0a47d1adea7d5fb7650f61d5a2910b9786088fc35e6d662, bytes: 1186, store: repo}
---

# M13 Cd-1 — clean-room G18 concurrency issue-compare

## Context

Closes Wave **Cd-1** of [[plan:root/20261109024635]] under the Issue-resolution Done rule ([[sot:root/program-roadmap]] v0.5.0): each claimed differentiate/lease ID reproduced in Docker — stock **`git`** exhibits pathology; **`arks`** meets lease-uplift resolved criteria; comparative receipt attached. Grant schema: [[adr:root/20261109032955]]. Does **not** redefine M9/M10/M11 exits. Does **not** start M14 / G19 perf work. Unlisted differentiate IDs remain Open for Cd-2+.

Attached: `receipt.json` (`arks-g018-concurrency-issue-compare-issue-compare/v1`, wave `Cd-1`).

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` services `g018-concurrency-issue-compare` (+ regression `g011-lease-product`, `g017-concurrency-issue-compare`, `g014-cli-parity-production`, `g016-journey-compare`) |
| Entrypoints | `verify/scripts/run-g018-concurrency-issue-compare.sh`, `run-g011-lease-product.sh`, `run-g017-concurrency-issue-compare.sh`, `run-g014-cli-parity-production.sh`, `run-g016-journey-compare.sh` |
| Image | `arks-verify-m1:0.1.0` |
| Image digest | `sha256:bab37bd1ea03832a5ca30aa41e598eafc8bef7a5b1026d65acefb75e3f4a73c1` |
| Stock Git (in-image) | `git version 2.39.5` |
| Tree SHA at G18 run | `2fcad39e353e8a44a0024e72b7093c3cd1676803` (pre-land tip) |
| Tree SHA (evidence tip) | `80ead67` |
| Host Docker engine | Linux (`desktop-linux`) |

## Cd-1 G18 results

| ID | git | arks | Notes |
|----|-----|------|-------|
| REF-001 | pathology_pass | resolved_pass | `refns:` lease coordinates tip writers; lease ≠ skip Git locks |
| REF-006 | pathology_pass | resolved_pass | Lease serializes tip writers before CAS; wait honesty for loser |
| REF-011 | pathology_pass | resolved_pass | Partitioned agent `refns:` namespaces; overlapping prefix AlreadyHeld |
| AGENT-003 | pathology_pass | resolved_pass | M13 lease AlreadyHeld wait/backoff; write path honors live locks |

## Regression

| Gate | Outcome |
|------|---------|
| G11 | PASS |
| G17 | PASS (Cm-1..Cm-3) |
| G14 | PASS |
| G16 functional | PASS (`ARKS_JOURNEY_ITERATIONS=1`; no ≥20% bar) |
| gix-exit / no-celir | PASS |

## Uplifts landed (Cd-1)

- ADR grant prefixes `refns:` / `index:` ([[adr:root/20261109032955]]).
- `arks lease` porcelain (grant/release/inspect/list/expire) over durable L2 store.
- G18 harness + compose service + matrix `verify/matrices/matrix-g018-concurrency-issue-compare.md`.

## Non-claims

- Does **not** redefine M9/M10/M11.
- No L3; no CELIR; G8 deferred.
- No ≥20% / M14 / G19.
- Unlisted differentiate IDs remain **Open** for Cd-2+.

## Reproduce

```bash
docker compose -f verify/docker-compose.yml run --rm g018-concurrency-issue-compare
docker compose -f verify/docker-compose.yml run --rm g011-lease-product
docker compose -f verify/docker-compose.yml run --rm g017-concurrency-issue-compare
docker compose -f verify/docker-compose.yml run --rm g014-cli-parity-production
docker compose -f verify/docker-compose.yml run --rm -e ARKS_JOURNEY_ITERATIONS=1 g016-journey-compare
spec check
```
