---
id: evidence:root/20261109045705
title: M16 Sy-2 G21 worktree submodule orphan
status: active
created: 2026-11-09T04:57:05Z
updated: 2026-11-09T04:58:44Z
tags: [g021-cli-flag-symmetry, m16, sy-2, clean-room, verified, complete-symmetry, worktree, submodule, orphan]
rel:
  depends_on: [plan:root/20261109043910, sot:root/cli-flag-parity-complete, sot:root/cli-surface, sot:root/interop-test-matrix, sot:root/program-roadmap, evidence:root/20261109044908]
  evidence: []
files:
  - {path: receipt.json, sha256: e57eeb676a7a1faaad2047fe344f82d91a1a034fecf14d96371fc759ad4445d9, bytes: 806, store: repo}
---

# M16 Sy-2 — G21 worktree / submodule recursive / orphan

Closes wave **Sy-2** of [[plan:root/20261109043910]] / [[sot:root/cli-flag-parity-complete]]: linked-worktree porcelain, submodule recursive/remote, and `checkout --orphan`.

Primary cites (pin Git 2.39): [git-worktree](https://git-scm.com/docs/git-worktree/2.39.0), [git-submodule](https://git-scm.com/docs/git-submodule/2.39.0), [git-checkout](https://git-scm.com/docs/git-checkout/2.39.0).

## Claim rows (PASS)

| ID | Focus |
|----|-------|
| M16-SY2-01 | `worktree list --porcelain`; `lock`/`unlock`; `add --detach` |
| M16-SY2-02 | `submodule update --init --recursive` / `status --recursive`; `update --remote` |
| M16-SY2-03 | `checkout --orphan` unborn branch; first commit has zero parents |

## Environment

| Field | Value |
|-------|-------|
| Compose | `g021-cli-flag-symmetry` |
| Image | `arks-verify-m1:0.1.0` / `sha256:bab37bd1ea03832a5ca30aa41e598eafc8bef7a5b1026d65acefb75e3f4a73c1` |
| Stock Git | `git version 2.39.5` |
| Host advisory | green (not verified) |
| G20 / G14 regression | PASS |

## Reproduce

```bash
docker compose -f verify/docker-compose.yml run --rm g021-cli-flag-symmetry
spec check
```

Tip SHA pinned after land commit (see receipt.json).
