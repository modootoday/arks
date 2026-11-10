---
id: evidence:root/20261109050238
title: M16 Sy-3 G21 plumbing breadth
status: active
created: 2026-11-09T05:02:38Z
updated: 2026-11-09T05:02:38Z
tags: [g021-cli-flag-symmetry, m16, sy-3, plumbing, clean-room, verified, complete-symmetry]
rel:
  depends_on:
    - plan:root/20261109043910
    - sot:root/cli-flag-parity-complete
    - sot:root/cli-surface
    - sot:root/interop-test-matrix
    - sot:root/program-roadmap
    - evidence:root/20261109045705
  evidence: []
files:
  - {path: receipt.json, sha256: 5cd9e7985464a0ff7bc3b2d3704dbf684a30235e7b3127b37b94197f19c9611c, bytes: 544, store: repo}
---

# M16 Sy-3 — G21 plumbing breadth

Closes wave **Sy-3** of [[plan:root/20261109043910]]: scripting plumbing breadth claimed implement (no production refuse leftovers on this set).

## Claim rows (PASS)

| ID | Focus |
|----|-------|
| M16-SY3-01 | `show-ref` / `for-each-ref` / `rev-list` / `ls-files` / `ls-tree` / `merge-base --all` / `read-tree` / `diff-tree` / `update-index` |
| M16-SY3-02 | `pack-objects` + `index-pack` round-trip; stock `git index-pack --verify` |
| Regression | G20 + G14 |

Also landed: `check-ignore` (minimal `.gitignore`), `name-rev` (minimal).

## Environment

| Field | Value |
|-------|-------|
| Compose | `g021-cli-flag-symmetry` |
| Image | `arks-verify-m1:0.1.0` / `sha256:bab37bd1ea03832a5ca30aa41e598eafc8bef7a5b1026d65acefb75e3f4a73c1` |
| Stock Git | `git version 2.39.5` |

## Reproduce

```bash
docker compose -f verify/docker-compose.yml run --rm g021-cli-flag-symmetry
spec check
```
