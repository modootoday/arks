---
id: sot:root/verify-cleanroom-symmetry-matrix
title: Clean-room symmetry matrix (three DoD pillars)
status: active
created: 2026-11-10T19:24:00Z
updated: 2026-11-13T05:30:29Z
tags: [verify, clean-room, docker, symmetry, dod, git, daemon, hub, gh, self-host, hosting, claimed-residual, single-release, no-invent-pass, optimized-dod]
verify: manual
rel:
  depends_on:
    - sot:root/product-verification-clean-room
    - sot:root/cli-git-help-matrix
    - sot:root/arks-daemon-git-command-matrix
    - sot:root/arks-hub-gh-command-matrix
    - sot:root/arks-hub-local-multi-agent-pr
    - sot:root/arks-self-host-server
    - sot:root/arks-claimed-residual-gates
    - sot:root/arks-dod-optimized
    - sot:root/verify-directory-conventions
    - sot:root/arks-daemon-product-intent
    - sot:root/arks-hub-product-intent
    - sot:root/cli-surface
    - sot:root/program-roadmap
    - sot:root/production-residual-register
    - research:root/20261111034805
    - research:root/20261111035226
    - research:root/20261111074523
    - research:root/20261113143029
  evidence:
    - evidence:root/20261111034600
    - evidence:root/20261111035226
  governs:
    - verify/matrices/cli-git-help-matrix.csv
    - verify/matrices/daemon-git-arks-e2e-matrix.csv
    - verify/matrices/hub-gh-arks-e2e-matrix.csv
    - verify/scripts/run-symmetry-dod.sh
    - verify/scripts/run-symmetry-git.sh
    - verify/scripts/run-symmetry-daemon.sh
    - verify/scripts/run-symmetry-hub.sh
    - verify/scripts/run-self-host-e2e.sh
    - verify/scripts/run-claimed-residual-dod.sh
---

# Clean-room symmetry matrix (three DoD pillars)

Version **0.1.6**. Umbrella **canon** for the public-story clean-room **Definition of Done**: three peer-symmetry pillars in **Linux Docker** (`arks-verify-m1:0.1.0`), plus **related hosting** e2e for arks-only self-host server (LAN/WAN) under [[sot:root/arks-self-host-server]], plus **adjacent Claimed residual** scaffolds under [[sot:root/arks-claimed-residual-gates]] (B1–B9 — **not** a fourth symmetry pillar). **Renewed must-pass / cut table:** [[sot:root/arks-dod-optimized]] (research [[research:root/20261113143029]]) — Partial×117 / hub terminal×6 / later×119 / OUT / SysV / L9 / G8 are **explicitly non-DoD**; bounded raises OD-UNIFIED / OD-SERVE-CFG / OD-COMMIT-HONESTY only. **No invent PASS/Claimed** from SoT alone. Directory / naming / SKIP·STRICT / out-path conventions: [[sot:root/verify-directory-conventions]].

**Published binary:** **`arks` only** (`arks daemon`, `arks hub …`). Internal crates OK. Product paths **MUST NOT** spawn `git` or `gh`. Stock `git` / surveyed `gh` are oracles only.

**Remote empty — no push.** Windows hosts drive compose; authority is Linux-in-container bash, not host `sh`.

### Tip currency (living tip ≥ `ea6571c`; optimized DoD)

| Pillar | Inventory honesty now | DoD vs non-DoD |
|--------|----------------------|----------------|
| **1. CLI** (`git`↔`arks`) | pin list-cmds **Missing = 0** (DoD judge clean-room **2.39.5**); living CSV Partial **117** / Refuse **47** | **DoD:** Missing=0 + OD-UNIFIED + OD-COMMIT-HONESTY. **Non-DoD:** Partial×117 remainder / Refuse |
| **2. Daemon** | living CSV **Missing = 0**; **Parity = 20**; Refuse/N/A **16**; Partial **0** ([[sot:root/arks-daemon-git-command-matrix]]) | **DoD:** Missing=0 + Parity carrying. **Non-DoD:** SysV / privilege-drop Refuse |
| **3. Hub/forge client** | v1 **Missing = 0**; Claimed **94** + terminal Partial **6** (OAuth/CI/git-spawn); later Deferred **119** OUT of v1 DoD; Refuse/N/A **28** | **DoD:** v1 Missing=0 + Claimed inventory. **Non-DoD:** terminal×6 keep; later×119; OUT Refuse |
| **Related: Self-host** | [[sot:root/arks-self-host-server]] + IW-FORGE-SERVE under `dod-self-host`; **OD-SERVE-CFG** smoke under optimized DoD | WAN “secure by default” forbidden; not a fourth pillar |
| **Adjacent: Claimed residual** | [[sot:root/arks-claimed-residual-gates]] — **B1/B2/B3/B4/B5/B5+/B7(HTTP+SSH+forge)/B9 Claimed** | not a fourth symmetry pillar |

Orchestrator wired: `verify/scripts/run-symmetry-dod.sh` / compose `dod-symmetry`. Related hosting: compose `dod-self-host` → `verify/scripts/run-self-host-e2e.sh`. Claimed residual: compose `dod-claimed-residual` → `verify/scripts/run-claimed-residual-dod.sh` (`ARKS_DOD_STRICT=1` fail-closed; **no invent PASS**). ARKS-native (non-peer) e2e inventory: [[sot:root/arks-native-e2e-checklist]]. Optimized PASS/FAIL: [[sot:root/arks-dod-optimized]].

## Rule

### Three pillars (normative)

| Pillar | Stock peer | Product entry | Inventory / intent SoT | Machine table(s) | Compose | Entrypoint |
|--------|------------|---------------|------------------------|------------------|---------|------------|
| **1. CLI** | `git` | `arks <cmd>` | [[sot:root/cli-git-help-matrix]] | `verify/matrices/cli-git-help-matrix.csv` | `dod-cli-matrix` | `verify/scripts/run-cli-git-arks-matrix-e2e.sh` |
| **2. Daemon** | `git-daemon` | `arks daemon` | [[sot:root/arks-daemon-git-command-matrix]] (+ intent [[sot:root/arks-daemon-product-intent]]) | `verify/matrices/daemon-git-arks-e2e-matrix.csv` | `dod-daemon-matrix` | `verify/scripts/run-daemon-git-arks-matrix-e2e.sh` |
| **3. Hub/forge** | `gh` | flat `arks pr|issue|…` (transitional `arks hub …`) | [[sot:root/arks-hub-gh-command-matrix]] + [[sot:root/arks-hub-local-multi-agent-pr]] | Design: `.spec/sot/arks-hub-gh-command-matrix.csv` · status: `verify/matrices/hub-gh-arks-e2e-matrix.csv` | `dod-hub-matrix` | `verify/scripts/run-hub-gh-arks-matrix-e2e.sh` |

Pillar 3 design CSV is the **gh leaf / phase** inventory the smoke gate counts today. The `verify/matrices/hub-gh-arks-e2e-matrix.csv` ledger tracks per-row `status_now` (Missing/Partial/…) for future strict e2e — **not Claimed** while Missing > 0.

### Orchestrator aliases

| Alias | Action |
|-------|--------|
| `verify/scripts/run-symmetry-git.sh` | pillar 1 |
| `verify/scripts/run-symmetry-daemon.sh` | pillar 2 |
| `verify/scripts/run-symmetry-hub.sh` | pillar 3 |
| `verify/scripts/run-symmetry-dod.sh` | all three sequentially (halt on hard FAIL only) |

```bash
docker compose -f verify/docker-compose.yml run --rm dod-cli-matrix
docker compose -f verify/docker-compose.yml run --rm dod-daemon-matrix
docker compose -f verify/docker-compose.yml run --rm dod-hub-matrix
docker compose -f verify/docker-compose.yml run --rm dod-symmetry
```

### Honesty law (ABSOLUTE)

1. **Smoke ≠ Claimed.** Default mode = help/CSV honesty smoke (`smoke-only PASS` / `SKIP` OK while Missing > 0).
2. **`ARKS_DOD_STRICT=1`** fails closed on unfinished must-implement rows / missing scenario SoT — **no invent PASS**.
3. Host cargo greens are advisory ([[sot:root/product-verification-clean-room]]).
4. These three pillars are the **public-story clean-room DoD** for git / daemon / hub symmetry. They do **not** reopen L9, live G8 latency SLO, or trimmed forge/host-env/LFS/CELIR-product Claimed ([[sot:root/production-residual-register]], [[sot:root/program-roadmap]]).
5. Hub multi-agent local-host load is **framework-first** ([[sot:root/arks-hub-local-multi-agent-pr]]); runners MAY be scaffold/TODO.
6. A future `arks agent` e2e pillar MAY join later — **not** a fourth symmetry DoD pillar until its own SoT + gate land (do not invent PASS from agent design SoTs).
7. **arks-only self-host server** (LAN/WAN product hosting) is **adjacent** to the three symmetry pillars — not a substitute for git-daemon flag parity or hub gh-matrix. See §Related hosting below.

### Related hosting — arks self-host server (LAN/WAN)

**Product story:** operators run a **GitHub-class** host **using only `arks`** (`arks serve` default HTTP+SSH+forge with `--no-http`/`--no-ssh`/`--no-forge`; `arks serve config`; `arks daemon` optional/legacy; pack stdio as applicable), reachable on localhost, **LAN**, and optionally **WAN**, without spawning `git`/`gh`. Owning SoT: [[sot:root/arks-self-host-server]] (active design; **≠ Claimed hosting e2e**; SSH + forge-in-serve Partial/fail-closed until Claimed).

| Item | Normative rule |
|------|----------------|
| Owning SoT | [[sot:root/arks-self-host-server]] |
| Status now | **Design** — SoT active (v0.2 umbrella); HTTP git wire Claimed elsewhere (G15/G29–G32/G36); SSH/forge-in-serve Missing/Partial; dedicated hosting e2e **scaffold** |
| Clean-room entry | compose `dod-self-host` → `verify/scripts/run-self-host-e2e.sh` (default SKIP/scaffold; `ARKS_DOD_STRICT=1` fails closed until scenario Claimed) |
| Relationship to pillar 2 | Daemon **flag symmetry** ([[sot:root/arks-daemon-git-command-matrix]]) ≠ full self-host product DoD (bind/auth/TLS-ops + forge module live in [[sot:root/arks-self-host-server]]) |
| Relationship to pillar 3 | Forge **client** remains logical gh-matrix (flat argv target) + multi-agent ([[sot:root/arks-hub-local-multi-agent-pr]]); local forge **host** is `arks serve` (transitional `hub serve` OK until alias) |
| Honesty | Smoke ≠ Claimed; WAN “secure by default” forbidden without TLS/ops + evidence per self-host SoT |

### Pin oracles vs host inventory (normative)

| Surface | Clean-room / survey **judge** | Host inventory (skew-only; not Claimed judge) |
|---------|-------------------------------|-----------------------------------------------|
| Git / git-daemon | Docker stock **`git version 2.39.5`** (`verify/images.lock.json`) | Host observed **`git version 2.56.0.windows.4`** — may show newer flags/help; **MUST NOT** override pin DoD |
| gh → hub matrix | Survey pin **`gh` 2.103.3** ([[sot:root/arks-hub-gh-command-matrix]]) | Same host **`gh` 2.103.3** (2026-10-26) as of this SoT amend; sibling MAY refresh CSV leaves |

Host upgrades are **inventory signals**. Authoritative CLI/daemon symmetry remains clean-room pin **2.39.5**. Hub leaf completeness tracks the surveyed `gh` pin in the design CSV (sibling refresh).

## Why

One umbrella ties exhaustive peer matrices to the same Docker entrypoints operators already run, keeping public honesty fail-closed about Missing while scaffolds stay usable.

## Verification

1. All three pillar SoTs + machine tables exist and are linked here.
2. Compose `dod-cli-matrix` / `dod-daemon-matrix` / `dod-hub-matrix` / `dod-symmetry` invoke the scripts above.
3. Default run never claims full DoD while Missing > 0; strict fails closed.
4. Hub scenario SoT defines N-agent local-host framework even if runners are scaffold.
5. Residual / roadmap cite these three pillars as clean-room public-story DoD — not as invented Claimed.
6. Related hosting [[sot:root/arks-self-host-server]] is linked; `dod-self-host` scaffold exists; hosting e2e Claimed only with evidence (no invent PASS).
7. Adjacent Claimed residual [[sot:root/arks-claimed-residual-gates]] is linked; `dod-claimed-residual` scaffold exists; B1–B9 Claimed only with evidence (no invent PASS; not a fourth symmetry pillar).
