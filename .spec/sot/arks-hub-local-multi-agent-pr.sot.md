---
id: sot:root/arks-hub-local-multi-agent-pr
title: arks hub local multi-agent PR/issue scenario framework
status: active
created: 2026-11-10T19:24:00Z
updated: 2026-11-25T05:10:00Z
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
  evidence:
    - evidence:root/20261125044100-hub-local-multi-agent
---

# arks hub local multi-agent PR/issue scenario framework

Version **0.3.0**. Normative **scenario framework** for large-scale agent collaboration against a **locally hosted** forge API (`arks serve` forge module; transitional `arks hub serve` until alias lands), using PR/issue/comment/label/review surfaces from [[sot:root/arks-hub-gh-command-matrix]]. Umbrella DoD pillar 3: [[sot:root/verify-cleanroom-symmetry-matrix]]. Host ownership: [[sot:root/arks-self-host-server]].

**Honesty:** this SoT defines the framework and phased Claimed path. It does **not** Claim hub feature completeness. Since [[evidence:root/20261125044100-hub-local-multi-agent]] (Docker, 6 and 12 agents, exit 0) every scenario row is Claimed except HUB-MA-09, which is Refuse/N/A with its reason.

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
| **HUB-MA-01** | N agents open distinct issues on one local repo | `issue create` / `issue list` / `issue view` | v1 | **Claimed** [[evidence:root/20261125044100-hub-local-multi-agent]] (concurrent creates get distinct numbers, authors kept) |
| **HUB-MA-02** | N agents comment on the same issue (contention) | `issue comment` / `issue view` | v1 | **Claimed** [[evidence:root/20261125044100-hub-local-multi-agent]] (every comment lands, ids unique, per-author order kept) |
| **HUB-MA-03** | N agents open PRs from divergent heads | `pr create` / `pr list` / `pr view` | v1 | **Claimed** [[evidence:root/20261125044100-hub-local-multi-agent]] (concurrent clone / branch / push / `pr create` over smart HTTP; files and commits per PR) |
| **HUB-MA-04** | Cross-agent PR review + comment load | `pr review` / `pr comment` / `pr checks` (stub OK) | v1 | **Claimed** [[evidence:root/20261125044100-hub-local-multi-agent]] (cross reviews and comments, commit-status checks, concurrent merges with one refused conflict resolved by its author) |
| **HUB-MA-05** | Label storm (add/remove under concurrency) | `label` / `pr edit` / `issue edit` | v1 | **Claimed** [[evidence:root/20261125044100-hub-local-multi-agent]] (one winner per label name; concurrent add / remove keeps every change, contract item 1) |
| **HUB-MA-06** | Release cut while agents still commenting | `release create` / `release view` + issue/pr read | later | **Claimed** [[evidence:root/20261125044100-hub-local-multi-agent]] (release race has one winner while agents comment; asset and tag consistent) |
| **HUB-MA-07** | Auth switch across agents (token isolation) | `auth login` / `auth status` / `auth token` | v1 | **Claimed** [[evidence:root/20261125044100-hub-local-multi-agent]] (per-agent `/user`, token and status; account switch; refused tokens; isolated logout) |
| **HUB-MA-08** | Search/status fan-out under load | `search` / `status` / `pr status` | later | **Claimed** [[evidence:root/20261125044100-hub-local-multi-agent]] (search / `pr status` / `status` fan-out stays exact under a concurrent writer) |
| **HUB-MA-09** | Actions run/workflow observe (local fixtures) | `run` / `workflow` | out-of-scope | **Refuse/N/A**: Actions are GitHub-platform-only and out of scope for the arks forge ([[adr:root/20261119230000]] item 3); the runner checks that every agent's `run` / `workflow` call refuses with exit 2 and reaches no forge [[evidence:root/20261125044100-hub-local-multi-agent]] |
| **HUB-MA-10** | Full v1 matrix soak — all mapped v1 leaves touched over time | entire v1 set in [[sot:root/arks-hub-gh-command-matrix]] | v1-framework | **Claimed** [[evidence:root/20261125044100-hub-local-multi-agent]] (every agent walks all 171 v1 targets of `verify/matrices/hub-gh-arks-e2e-matrix.csv` in its own repositories, all at once, over 2 and 3 rounds; ledger 171/171) |

### Phase meanings

| Phase | Meaning |
|-------|---------|
| **v1** | Required scenario class for a useful local multi-agent collaboration slice once hub serve + mapped commands exist |
| **v1-framework** | Must be **defined** in this SoT for public-story completeness even if runners are incomplete |
| **later** | Keep ID; defer runner until after v1 slice Claimed |
| **out-of-scope** | Excluded by a decision record; the row stays with disposition Refuse/N/A and its reason |

### Disposition vocabulary

| Disposition | Meaning |
|-------------|---------|
| **Scaffold** | Scenario ID + topology + entrypoint TODO exist; not Verified (no row is Scaffold since [[evidence:root/20261125044100-hub-local-multi-agent]]) |
| **Claimed** | Clean-room Docker receipt proves the scenario against local `arks serve` forge module (or documented transitional `arks hub serve` until alias removed) |
| **Refuse/N/A** | Explicit permanent exclusion (requires SoT amend + reason) |

### Concurrency contract (agents sharing one forge)

1. **List edits are deltas.** `issue edit` and `pr edit` apply `--add-label` / `--remove-label`,
   `--add-assignee` / `--remove-assignee` (and, on `pr edit`, `--add-reviewer` / `--remove-reviewer`)
   as server-side deltas, removals first and then additions:
   `POST|DELETE …/{issues|pulls}/{n}/labels[/{name}]`, `POST|DELETE …/{issues|pulls}/{n}/assignees`,
   `POST|DELETE …/pulls/{n}/requested_reviewers`. They never read the whole set and write it back,
   so concurrent editors never drop each other's changes. Removing a label the item does not carry is
   a no-op. The issue endpoints follow GitHub REST ([REST issue labels](https://docs.github.com/en/rest/issues/labels),
   [REST assignees](https://docs.github.com/en/rest/issues/assignees), [REST review requests](https://docs.github.com/en/rest/pulls/review-requests));
   the `pulls/{n}/labels` and `pulls/{n}/assignees` forms are the arks forge's equivalents, because the
   arks forge numbers pull requests apart from issues. gh likewise edits labels through add / remove
   mutations ([GraphQL `addLabelsToLabelable` / `removeLabelsFromLabelable`](https://docs.github.com/en/graphql/reference/mutations)).
2. **Distinct numbers.** Concurrent `issue create`, `pr create` and comment posts each get their own
   number or id; none is lost or duplicated.
3. **One winner per name.** Concurrent `label create` of one name succeeds once; the others fail (exit 1).
4. **Merges see the moved base.** Concurrent `pr merge` calls apply one at a time; a pull whose change
   conflicts with what landed first is refused as not mergeable until its author resolves it.
5. **Per-account data stays with its account.** `@me` and `/user/…` resolve to the account of the
   request's token, not to the forge's default user: `/user/keys`, `/user/ssh_signing_keys` and
   `/user/gpg_keys` list, show and delete only that account's keys (a key is still unique across the
   forge: adding one another account holds fails as "already in use"), as in GitHub REST
   ([user SSH keys](https://docs.github.com/en/rest/users/keys), [user GPG keys](https://docs.github.com/en/rest/users/gpg-keys));
   `/gists` lists the account's own gists, a new gist belongs to its creator, and only the owner
   edits or deletes it ([REST gists](https://docs.github.com/en/rest/gists/gists)); `/users/@me/projectsV2`
   is the account's own projects. Data written before accounts were recorded belongs to the forge's
   default user.

### Entrypoints

| Artifact | Role |
|----------|------|
| `verify/scripts/run-hub-gh-arks-matrix-e2e.sh` | Pillar 3 gate; detects this SoT; does **not** invent scenario PASS |
| `verify/scripts/run-hub-local-multi-agent.sh` | Sole runner: builds or takes `ARKS_BIN`, runs the driver `verify/scripts/hub-local-multi-agent.py` (SMOKE, HUB-MA-01…10; `ARKS_HUB_MA_AGENTS`, `ARKS_HUB_MA_SCENARIOS`, `ARKS_HUB_MA_SOAK_ROUNDS`), writes logs, `summary.json` and `receipt.json` to `ARKS_HUB_MA_OUT`; exit 1 on any failed scenario; SKIP only when this SoT is missing |
| compose `dod-hub-matrix` | Authoritative venue for pillar 3 smoke and for the multi-agent runner (entrypoint replaced, as in [[evidence:root/20261125044100-hub-local-multi-agent]]) |

### Non-goals

1. Claiming GitHub live scale / Checks merge-queue parity.
2. Spawning `gh` or `git` from product or from scenario runners’ product path.
3. Requiring hub scenarios for M9/M16 git alias DoD.
4. Inventing PASS from this SoT text alone.

## Why

Large-scale agent collaboration is the differentiating hub story. Defining N-agent local-host scenarios now keeps the v1 matrix honest about “all hub features over time” without pretending runners already Claimed.

## Verification

1. Every HUB-MA-* ID above is listed with phase + disposition.
2. Pillar 3 script references this SoT path; the runner fails closed (exit 1) on any failed scenario, never SKIP-as-PASS.
3. Claimed requires clean-room evidence id linked from residual/roadmap.
4. No separate `arks-hub` binary in the topology diagram or public copy.
