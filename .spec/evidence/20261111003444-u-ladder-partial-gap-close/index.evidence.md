---
id: evidence:root/20261111003444
title: U-ladder partial-gap close (Pf5 amortize + host-env deepen + forge thin-wrap)
status: active
created: 2026-11-10T15:34:44Z
updated: 2026-11-10T15:34:44Z
tags:
  [
    u-ladder,
    pf5,
    host-env,
    fs-007,
    fs-014,
    win-003,
    forge,
    g8-local-forge,
    l9,
    sha1cd,
    partial-completion,
  ]
rel:
  depends_on:
    - research:root/20261111001816
    - sot:root/program-roadmap
    - sot:root/pf5-journey-seed-amortization
    - adr:root/20261110162000
    - evidence:root/20261110224814
    - evidence:root/20261110222100
    - evidence:root/20261110221919
  evidence: []
files:
  - host/g-drivefs-probe.json
---

# U-ladder partial-gap close

Single controlled wave after tip ≥ `307d509` per [[research:root/20261111001816]].

## U-ladder dispositions (this pack)

| Step | Gap | Disposition |
|------|-----|-------------|
| **U1** Pf5 ALIAS/REVERT | Sole G19 remasure (`g19/`): M17 **19/19**; ≤0.5× claimed=17 + `aggressive_na` ALIAS **0.7906** (ALIAS residual 48→56 **hurt** PATH tax — rolled back to 48) + REVERT **0.5417** (kept +12 amortize; improved from 0.67). **Claimed(partial)** — no second remasure |
| **U2** FS-007 / WIN-003 / FS-014 | Deepened detect + opt-in refuse-sync + LSM sysfs detect + errno honesty; G: DriveFS host advisory re-probe (`host/g-drivefs-probe.json`); full sync/hydration/LSM-bypass still Impossible |
| **U3** Live G8 latency | Secrets `g8-live` **total_count=0** → local-forge wire refresh only; **no** invent latency PASS |
| **U4** L9 | Owner-blocked; reopen pack filed under research — **no** publish |
| **U5** Forge UX | `arks forge` thin `gh` wrap + `recipe` — **Claimed-closest**; full forge host UX stays Non-goal→Partial(closest) |
| **U6** sha1cd immunity | Detect Claimed (prior); immunity remains **Impossible** (docs only) |

## Honesty

- No invent PASS for secrets-absent live G8.
- No unauthorized crates.io publish.
- No G19 stampede after a real gate FAIL.
- DriveFS G: ≠ WIN-003 OneDrive placeholders.
