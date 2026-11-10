---
id: plan:root/20261111042400
title: Hub local multi-agent PR/issue scenario scaffolding
status: active
created: 2026-11-10T19:24:00Z
updated: 2026-11-10T19:26:00Z
tags: [arks-hub, multi-agent, clean-room, scenario, scaffold, phased, no-invent-claimed]
verify: manual
rel:
  depends_on:
    - sot:root/arks-hub-local-multi-agent-pr
    - sot:root/verify-cleanroom-symmetry-matrix
    - sot:root/arks-hub-product-intent
    - sot:root/arks-hub-gh-command-matrix
  implements:
    - sot:root/arks-hub-local-multi-agent-pr
---

# Hub local multi-agent PR/issue scenario scaffolding

## Goal

Land the **scenario framework** for N agents collaborating via **local** `arks hub serve` (PR/issue/comment/label/review) under clean-room DoD pillar 3. Phased Claimed vs scaffold — **no invent PASS**.

## Directive

1. SoT [[sot:root/arks-hub-local-multi-agent-pr]] owns scenario IDs **HUB-MA-01…10**.
2. Pillar gate `dod-hub-matrix` / `run-hub-gh-arks-matrix-e2e.sh` detects the scenario SoT; default smoke does not Claim scenarios.
3. Runner `verify/scripts/run-hub-local-multi-agent.sh` starts as **SKIP/TODO** until `arks hub serve` + mapped v1 commands exist.
4. Never spawn `gh`/`git` on product paths.
5. Single binary `arks` only.

## Phases

| Phase | Deliverable | Disposition |
|-------|-------------|-------------|
| **P0** | Scenario SoT + umbrella link + SKIP runner | **Done when this plan + SoT land** |
| **P1** | Local `arks hub serve` loopback smoke (1 agent) | Scaffold → Claimed with evidence |
| **P2** | HUB-MA-01…05 concurrent N-agent load | Scaffold → Claimed per ID |
| **P3** | HUB-MA-06…10 + soak (HUB-MA-10) | later / framework |

## Acceptance (P0)

| Check | Done when |
|-------|-----------|
| Scenario SoT active | HUB-MA-* catalog present |
| Umbrella cites scenario | [[sot:root/verify-cleanroom-symmetry-matrix]] |
| Runner honesty | `run-hub-local-multi-agent.sh` exits 0 with SKIP/TODO (no fake PASS) |
| Gate wiring | hub e2e script knows scenario path; STRICT requires SoT |

## Non-goals

Full hub feature Claimed; live GitHub scale; inventing PASS; separate `arks-hub` bin.
