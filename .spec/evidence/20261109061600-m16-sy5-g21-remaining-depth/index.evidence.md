---
id: evidence:root/20261109061600
title: M16 Sy-5 G21 remaining depth + ancillary interrogators
status: active
created: 2026-11-09T06:16:00Z
updated: 2026-11-09T06:16:00Z
tags: [g021-cli-flag-symmetry, m16, sy-5, clean-room, verified, complete-symmetry]
rel:
  depends_on:
    - plan:root/20261109043910
    - sot:root/cli-flag-parity-complete
    - sot:root/cli-surface
    - sot:root/interop-test-matrix
    - sot:root/program-roadmap
    - evidence:root/20261109054500
  evidence: []
files:
  - {path: receipt.json, sha256: 3932aafc2ee755bf711c82da3a86a7ef6e8f97e29371275b975f28c91238eeba, bytes: 585, store: repo}
---

# M16 Sy-5 — G21 remaining depth + ancillary interrogators

Closes wave **Sy-5** of [[plan:root/20261109043910]]: stash show/apply/drop;
cherry-pick `A..B` clean ranges; bisect run; notes append; blame path; fsck /
count-objects -v / pack-refs --all; detached HEAD commit/amend; cherry-pick
conflict markers + `--continue`/`--abort`. Prefer-also: range-diff `A...B`,
sparse-checkout disable/reapply.

## Claim rows (PASS)

| ID | Focus |
|----|-------|
| M16-SY5-01 | `stash show`/`apply`/`drop`; cherry-pick `A..B` clean range |
| M16-SY5-02 | `bisect run`; `notes append`; `blame` path; `fsck` / `count-objects -v` / `pack-refs --all` |
| M16-SY5-03 | Detached `commit` / `commit --amend`; sequencer `--continue`/`--abort` on conflicted cherry-pick |
| Regression | G20 + G14 |

## Environment

| Field | Value |
|-------|-------|
| Compose | `g021-cli-flag-symmetry` |
| Image | `arks-verify-m1:0.1.0` / `sha256:5736c5687d3b5775e187ca3d21fbc5b2a5a2bf8965c5381778ee478eb185adf0` |
| Stock Git | `git version 2.39.5` |

## Reproduce

```bash
docker compose -f verify/docker-compose.yml build toolchain
docker compose -f verify/docker-compose.yml run --rm g021-cli-flag-symmetry
spec check
```
