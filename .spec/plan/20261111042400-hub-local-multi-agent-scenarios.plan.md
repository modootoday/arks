---
id: plan:root/20261111042400
title: Hub local multi-agent PR/issue scenario scaffolding
status: archived
created: 2026-11-10T19:24:00Z
updated: 2026-11-25T05:15:00Z
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

**Status (2026-11-25, queue Q15 + Q16):** active → `archived` (**done**) — P1–P3 closed by the Docker receipt [[evidence:root/20261125044100-hub-local-multi-agent]] (tip `ec4bf43`, 6 and 12 agents, exit 0): SMOKE and HUB-MA-01…08 and 10 Claimed, HUB-MA-09 Refuse/N/A ([[adr:root/20261119230000]] item 3); `verify/scripts/run-hub-local-multi-agent.sh` runs every row and fails closed.

**Status (2026-11-25, queue Q14 audit):** stays **active**: P0 is done (scenario SoT, umbrella link, SKIP runner), but every HUB-MA-* row in [[sot:root/arks-hub-local-multi-agent-pr]] is still Scaffold and `verify/scripts/run-hub-local-multi-agent.sh` only prints SKIP; P1–P2 are queue unit Q15 and P3 is Q16 of [[plan:root/20261124000000]].

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
| **P1** | Local `arks hub serve` loopback smoke (1 agent) | **Done**: SMOKE against `arks serve` [[evidence:root/20261125044100-hub-local-multi-agent]] |
| **P2** | HUB-MA-01…05 concurrent N-agent load | **Done**: HUB-MA-01…05 and 07 Claimed [[evidence:root/20261125044100-hub-local-multi-agent]] |
| **P3** | HUB-MA-06…10 + soak (HUB-MA-10) | **Done**: 06, 08, 10 Claimed; 09 Refuse/N/A [[evidence:root/20261125044100-hub-local-multi-agent]] |

## Acceptance (P0)

| Check | Done when |
|-------|-----------|
| Scenario SoT active | HUB-MA-* catalog present |
| Umbrella cites scenario | [[sot:root/verify-cleanroom-symmetry-matrix]] |
| Runner honesty | `run-hub-local-multi-agent.sh` exits 0 with SKIP/TODO (no fake PASS) |
| Gate wiring | hub e2e script knows scenario path; STRICT requires SoT |

## Non-goals

Full hub feature Claimed; live GitHub scale; inventing PASS; separate `arks-hub` bin.
