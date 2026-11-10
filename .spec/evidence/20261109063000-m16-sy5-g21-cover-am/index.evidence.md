---
id: evidence:root/20261109063000
title: M16 Sy-5 G21 cover-letter and am --3way
status: active
created: 2026-11-09T06:30:00Z
updated: 2026-11-09T22:48:31Z
tags: [g21, m16, sy-5, clean-room, verified, complete-symmetry, cover-letter, am]
rel:
  depends_on: [plan:root/20261109043910, sot:root/cli-flag-parity-complete, sot:root/interop-test-matrix, evidence:root/20261109061600]
  evidence: []
files:
  - {path: receipt.json, sha256: 88b60ac669d6a2df852747dc9a3996786d2c4b7157529e6f4dac025c92eae158, bytes: 633, store: repo}
---

# M16 Sy-5 follow-up — format-patch --cover-letter / am --3way

Closes remaining Sy-5 SoT claimed rows under [[plan:root/20261109043910]] /
[[sot:root/cli-flag-parity-complete]] that were not G21-gated in
[[evidence:root/20261109061600]].

## Claim rows (PASS)

| ID | Focus |
|----|-------|
| M16-SY5-04 | `format-patch --cover-letter`; `am --3way` when blobs resolvable |
| Regression | Sy-1..Sy-5 prior rows + G20 + G14 |

## Environment

| Field | Value |
|-------|-------|
| Compose | `g21` |
| Image | `arks-verify-m1:0.1.0` / `sha256:5736c5687d3b5775e187ca3d21fbc5b2a5a2bf8965c5381778ee478eb185adf0` |
| Stock Git | `git version 2.39.5` |
| Land tip | `f52130b` (pin follow-up) |

## Reproduce

```bash
docker compose -f verify/docker-compose.yml run --rm g21
spec check
```
