---
id: evidence:root/20261110114000
title: S6 symmetry e2e g38/g39/g40 PASS
status: active
created: 2026-11-10T11:40:00Z
updated: 2026-11-10T11:40:00Z
tags: [s6, symmetry, e2e, g38, g39, g40, clean-room, docker]
rel:
  depends_on:
    - research:root/20261110105436
    - sot:root/interop-test-matrix
    - evidence:root/20261110110800
    - evidence:root/20261110105855
    - evidence:root/20261110112600
  evidence: []
files: []
---

# S6 — Symmetry e2e (g38 / g39 / g40) PASS

Venue: Docker `arks-verify-m1:0.1.0` sequential `compose run --rm --no-deps`. Entrypoint: `bash verify/scripts/run-s6-symmetry.sh`. Matrix: `verify/matrices/matrix-s6-symmetry.md`.

| Gate | Surface | Outcome |
|------|---------|---------|
| L8 (`g14`→`g16`→`g20`→`g21`) | Prior git↔arks e2e | **Cited Done** (`matrix-l8-e2e.md`; not re-stamped this run) |
| `g38` | R4 OpenPGP / X.509 / `tag -s` | **PASS** (3 tests) |
| `g39` | R3 `blob:none` twin | **PASS** (2 tests) |
| `g40` | S3 dumb HTTP deletes twin | **PASS** (1 test) |

Summary file: `verify/gates/g40/out/s6-summary.txt` (`g38=PASS` / `g39=PASS` / `g40=PASS`). Stock Git **2.39.5**, rustc **1.99.1**. No host cargo/gpg stampede; no invent PASS.
