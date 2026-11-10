---
id: evidence:root/20261110114000
title: S6 symmetry e2e g038-gpgsig-openpgp/g039-partial-clone-filter/g040-dumb-http-push-deletes PASS
status: active
created: 2026-11-10T11:40:00Z
updated: 2026-11-10T11:40:00Z
tags: [s6, symmetry, e2e, g038-gpgsig-openpgp, g039-partial-clone-filter, g040-dumb-http-push-deletes, clean-room, docker]
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

# S6 — Symmetry e2e (g038-gpgsig-openpgp / g039-partial-clone-filter / g040-dumb-http-push-deletes) PASS

Venue: Docker `arks-verify-m1:0.1.0` sequential `compose run --rm --no-deps`. Entrypoint: `bash verify/scripts/run-s6-symmetry.sh`. Matrix: `verify/matrices/matrix-s6-symmetry.md`.

| Gate | Surface | Outcome |
|------|---------|---------|
| L8 (`g014-cli-parity-production`→`g016-journey-compare`→`g020-cli-symmetric-parity`→`g021-cli-flag-symmetry`) | Prior git↔arks e2e | **Cited Done** (`matrix-l8-e2e.md`; not re-stamped this run) |
| `g038-gpgsig-openpgp` | R4 OpenPGP / X.509 / `tag -s` | **PASS** (3 tests) |
| `g039-partial-clone-filter` | R3 `blob:none` twin | **PASS** (2 tests) |
| `g040-dumb-http-push-deletes` | S3 dumb HTTP deletes twin | **PASS** (1 test) |

Summary file: `verify/gates/g040-dumb-http-push-deletes/out/s6-summary.txt` (`g038-gpgsig-openpgp=PASS` / `g039-partial-clone-filter=PASS` / `g040-dumb-http-push-deletes=PASS`). Stock Git **2.39.5**, rustc **1.99.1**. No host cargo/gpg stampede; no invent PASS.
