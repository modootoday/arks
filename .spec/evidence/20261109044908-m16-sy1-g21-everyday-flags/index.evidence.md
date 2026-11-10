---
id: evidence:root/20261109044908
title: M16 Sy-1 G21 everyday porcelain flags
status: active
created: 2026-11-09T04:49:08Z
updated: 2026-11-09T04:58:26Z
tags: [g021-cli-flag-symmetry, m16, sy-1, clean-room, verified, complete-symmetry]
rel:
  depends_on: [plan:root/20261109043910, sot:root/cli-flag-parity-complete, sot:root/cli-surface, sot:root/interop-test-matrix, sot:root/program-roadmap]
  evidence: []
files:
  - {path: receipt.json, sha256: 77090d45745440b4de9a57d86b1dcc5ae348e156daa426175569e28b03d53139, bytes: 794, store: repo}
---

# M16 Sy-1 — G21 everyday porcelain flags

Closes wave **Sy-1** of [[plan:root/20261109043910]] / [[sot:root/cli-flag-parity-complete]]: everyday porcelain flag gaps become **implement** (no production refuse leftovers on this claim set).

## Claim rows (PASS)

| ID | Focus |
|----|-------|
| M16-SY1-01 | `status -b` / porcelain `##` |
| M16-SY1-02 | `log --oneline` / `--graph` |
| M16-SY1-03 | `branch -a` / `-r` |
| M16-SY1-04 | `add --chmod=+x` |
| M16-SY1-05 | `clean -x` / `describe --dirty` |
| M16-SY1-06 | `switch --detach` / `diff --stat` / `init --bare` |
| M16-SY1-07 | G14 + G20 regression |

## Environment

| Field | Value |
|-------|-------|
| Compose | `g021-cli-flag-symmetry` |
| Image | `arks-verify-m1:0.1.0` / `sha256:bab37bd1ea03832a5ca30aa41e598eafc8bef7a5b1026d65acefb75e3f4a73c1` |
| Stock Git | `git version 2.39.5` |
| Host advisory | green (not verified) |

## Reproduce

```bash
docker compose -f verify/docker-compose.yml run --rm g021-cli-flag-symmetry
spec check
```

Tip SHA pinned after land commit (see receipt.json).
