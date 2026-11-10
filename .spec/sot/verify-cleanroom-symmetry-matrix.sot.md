---
id: sot:root/verify-cleanroom-symmetry-matrix
title: Clean-room symmetry matrix (three DoD pillars)
status: active
created: 2026-11-10T19:24:00Z
updated: 2026-11-10T22:50:00Z
tags: [verify, clean-room, docker, symmetry, dod, git, daemon, hub, gh, self-host, hosting, claimed-residual, single-release, no-invent-pass]
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
    - sot:root/verify-directory-conventions
    - sot:root/arks-daemon-product-intent
    - sot:root/arks-hub-product-intent
    - sot:root/cli-surface
    - sot:root/program-roadmap
    - sot:root/production-residual-register
    - research:root/20261111034805
    - research:root/20261111035226
    - research:root/20261111074523
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

Version **0.1.5**. Umbrella **canon** for the public-story clean-room **Definition of Done**: three peer-symmetry pillars in **Linux Docker** (`arks-verify-m1:0.1.0`), plus **related hosting** e2e for arks-only self-host server (LAN/WAN) under [[sot:root/arks-self-host-server]], plus **adjacent Claimed residual** scaffolds under [[sot:root/arks-claimed-residual-gates]] (B1–B9 — **not** a fourth symmetry pillar). **No invent PASS/Claimed** from SoT alone. Directory / naming / SKIP·STRICT / out-path conventions: [[sot:root/verify-directory-conventions]].

**Published binary:** **`arks` only** (`arks daemon`, `arks hub …`). Internal crates OK. Product paths **MUST NOT** spawn `git` or `gh`. Stock `git` / surveyed `gh` are oracles only.

**Remote empty — no push.** Windows hosts drive compose; authority is Linux-in-container bash, not host `sh`.

### Tip currency (SoT tip `c297e34` + local inventory refresh)

| Pillar | Inventory honesty now | Remaining gap (not invent PASS) |
|--------|----------------------|----------------------------------|
| **1. CLI** (`git`↔`arks`) | **pin** list-cmds **Missing = 0** (normative DoD = clean-room **2.39.5**); host **2.56** skew cmds tracked as Partial inventory reference only | Partial man-page flag depth; host skew ≠ pin advance / ≠ Claimed |
| **2. Daemon** | former Missing~25 → living CSV **Missing = 0** via Partial accept + Refuse/N/A | deepen Partial → Parity ([[sot:root/arks-daemon-git-command-matrix]]) |
| **3. Hub/forge client** | Logical **gh↔arks** pillar; target argv = **flat** `arks pr|issue|auth|…` ([[sot:root/arks-hub-product-intent]] v0.4); today Partial often via transitional `arks hub …` + `hub serve`; host = `arks serve` forge module | forge REST Claimed + multi-agent runners ([[sot:root/arks-hub-local-multi-agent-pr]]) |
| **Related: Self-host** | Design SoT **active** ([[sot:root/arks-self-host-server]] v0.2 — GitHub-class umbrella: HTTP+SSH+forge); wire receipts G15/G29–G32/G36 exist; forge-in-serve Missing/Partial | dedicated LAN-shaped hosting e2e (compose `dod-self-host`) — **scaffold / not Claimed** |
| **Adjacent: Claimed residual** | Design SoT **active** ([[sot:root/arks-claimed-residual-gates]] v0.1 — B1/B2/B3/B4/B5/B5+/B7/B9 after Partial wiring) | compose `dod-claimed-residual` (+ per-gate `dod-claimed-*`) — **SKIP/scaffold / not Claimed** |

Orchestrator wired: `verify/scripts/run-symmetry-dod.sh` / compose `dod-symmetry`. Related hosting smoke: compose `dod-self-host` → `verify/scripts/run-self-host-e2e.sh` (SKIP/scaffold until Claimed). Claimed residual scaffolds: compose `dod-claimed-residual` → `verify/scripts/run-claimed-residual-dod.sh` (SKIP until Claimed; **no invent PASS**).

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
