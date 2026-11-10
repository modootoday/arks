---
id: sot:root/arks-hub-local-multi-agent-pr
title: arks hub local multi-agent PR/issue scenario framework
status: active
created: 2026-11-10T19:24:00Z
updated: 2026-11-11T04:55:00Z
tags: [arks-hub, multi-agent, pr, issue, local-host, serve, clean-room, scenario, phased, no-invent-claimed]
verify: manual
rel:
  depends_on:
    - sot:root/verify-cleanroom-symmetry-matrix
    - sot:root/arks-hub-product-intent
    - sot:root/arks-hub-gh-command-matrix
    - sot:root/arks-hub-cli-surface
    - sot:root/arks-self-host-server
    - plan:root/20261111042400
  evidence: []
---

# arks hub local multi-agent PR/issue scenario framework

Version **0.1.1**. Normative **scenario framework** for large-scale agent collaboration against a **locally hosted** forge API (`arks serve` forge module; transitional `arks hub serve` until alias lands), using PR/issue/comment/label/review surfaces from [[sot:root/arks-hub-gh-command-matrix]]. Umbrella DoD pillar 3: [[sot:root/verify-cleanroom-symmetry-matrix]]. Host ownership: [[sot:root/arks-self-host-server]].

**Honesty:** this SoT defines the framework and phased Claimed path. It does **not** Claim hub feature completeness. Runners MAY be scaffold/TODO until evidence lands.

## Rule

### Topology (normative intent)

```text
┌─────────────────────────────────────────────┐
│  Linux clean-room Docker                     │
│                                              │
│   arks serve [… forge module …]              │
│     (transitional: arks hub serve)           │
│         ▲                                    │
│         │ native HTTPS/HTTP (no gh/git spawn)│
│   ┌─────┴──────┬──────────┬──────────┐       │
│   │ agent-01   │ agent-02 │ … agent-N│       │
│   │ arks pr…   │ arks …   │ flat CLI │       │
│   └────────────┴──────────┴──────────┘       │
└─────────────────────────────────────────────┘
```

1. **Local host:** forge API is served **inside** the clean-room via **`arks serve`** (or transitional `arks hub serve` / compose sidecar). Live GitHub is **out of** this framework’s Claimed path (G8/G28 remain ops/live harnesses).
2. **N agents:** concurrent CLI processes (or in-process test harnesses) each invoke **flat forge leaves** (`arks pr …`, …; transitional `arks hub …` OK) — never `gh`/`git` subprocess.
3. **Single release:** agents use the same `arks` binary under test.
4. **No invent PASS:** scenario rows start as **Scaffold** until a clean-room receipt marks **Claimed**.

### Scenario ID catalog (v1 framework)

| Scenario ID | Intent | Hub surfaces (min) | Phase | Disposition today |
|-------------|--------|--------------------|-------|-------------------|
| **HUB-MA-01** | N agents open distinct issues on one local repo | `issue create` / `issue list` / `issue view` | v1 | **Scaffold** |
| **HUB-MA-02** | N agents comment on the same issue (contention) | `issue comment` / `issue view` | v1 | **Scaffold** |
| **HUB-MA-03** | N agents open PRs from divergent heads | `pr create` / `pr list` / `pr view` | v1 | **Scaffold** |
| **HUB-MA-04** | Cross-agent PR review + comment load | `pr review` / `pr comment` / `pr checks` (stub OK) | v1 | **Scaffold** |
| **HUB-MA-05** | Label storm (add/remove under concurrency) | `label` / `pr edit` / `issue edit` | v1 | **Scaffold** |
| **HUB-MA-06** | Release cut while agents still commenting | `release create` / `release view` + issue/pr read | later | **Scaffold** |
| **HUB-MA-07** | Auth switch across agents (token isolation) | `auth login` / `auth status` / `auth token` | v1 | **Scaffold** |
| **HUB-MA-08** | Search/status fan-out under load | `search` / `status` / `pr status` | later | **Scaffold** |
| **HUB-MA-09** | Actions run/workflow observe (local fixtures) | `run` / `workflow` | later | **Scaffold** |
| **HUB-MA-10** | Full v1 matrix soak — all mapped v1 leaves touched over time | entire v1 set in [[sot:root/arks-hub-gh-command-matrix]] | v1-framework | **Scaffold** (umbrella soak; phased feature Claimed elsewhere) |

### Phase meanings

| Phase | Meaning |
|-------|---------|
| **v1** | Required scenario class for a useful local multi-agent collaboration slice once hub serve + mapped commands exist |
| **v1-framework** | Must be **defined** in this SoT for public-story completeness even if runners are incomplete |
| **later** | Keep ID; defer runner until after v1 slice Claimed |

### Disposition vocabulary

| Disposition | Meaning |
|-------------|---------|
| **Scaffold** | Scenario ID + topology + entrypoint TODO exist; not Verified |
| **Claimed** | Clean-room Docker receipt proves the scenario against local `arks serve` forge module (or documented transitional `arks hub serve` until alias removed) |
| **Refuse/N/A** | Explicit permanent exclusion (requires SoT amend + reason) |

### Entrypoint plan (scaffold)

| Artifact | Role |
|----------|------|
| `verify/scripts/run-hub-gh-arks-matrix-e2e.sh` | Pillar 3 gate; detects this SoT; does **not** invent scenario PASS |
| `verify/scripts/run-hub-local-multi-agent.sh` | Future sole runner (TODO until implement); MUST SKIP honestly if serve/binary incomplete |
| compose `dod-hub-matrix` | Authoritative venue for pillar 3 smoke; multi-agent load MAY be a later compose service |

### Non-goals

1. Claiming GitHub live scale / Checks merge-queue parity.
2. Spawning `gh` or `git` from product or from scenario runners’ product path.
3. Requiring hub scenarios for M9/M16 git alias DoD.
4. Inventing PASS from this SoT text alone.

## Why

Large-scale agent collaboration is the differentiating hub story. Defining N-agent local-host scenarios now keeps the v1 matrix honest about “all hub features over time” without pretending runners already Claimed.

## Verification

1. Every HUB-MA-* ID above is listed with phase + disposition.
2. Pillar 3 script references this SoT path; missing runner → SKIP/TODO, not PASS.
3. Claimed requires clean-room evidence id linked from residual/roadmap.
4. No separate `arks-hub` binary in the topology diagram or public copy.
