---
id: adr:root/20261128035000
title: Adopt agy parallel-development governance, task-scoped branch taxonomy and GitHub-native control plane
status: active
created: 2026-11-28T03:50:00Z
updated: 2026-11-28T03:50:00Z
tags: [governance, agy, parallelism, branches, pull-requests, autonomy, integration]
rel:
  depends_on:
    - sot:root/agent-governance
    - sot:root/autonomous-operation
    - adr:root/20261119220000
    - adr:root/20261124000000
    - research:root/20261128034500
---

# Adopt `agy` parallel-development governance, task-scoped branch taxonomy and GitHub-native control plane

## Context

The 2026-11-28 repository audit ([[research:root/20261128034500]]) found that:

1. All 73 merged pull requests (#1–#74, excluding issue #45) reused a single branch (`claude/kind-faraday-uvczj4`), matching the sequential single-branch assumption of [[sot:root/autonomous-operation]] v2.0.0.
2. When concurrent sessions ran in parallel on 2026-11-25, two finished branches (`claude/dazzling-knuth-l8m1nz`, +9 commits; `claude/happy-volta-8xfk6m`, +6 commits) were pushed to `origin` without opening pull requests and drifted 152 commits behind `main` without being noticed.
3. Prior unattended governance coupled execution to Claude Code (`.claude/settings.json` hooks, `claude/*`-only branch scope) and local `playpilot` fleet tooling (`playpilot mail`, `playpilot sync`, `playpilot claim`), which are not installed in **`agy` (Antigravity)** remote or desktop sessions.
4. Running unconstrained concurrent PRs against `main` causes hot-file merge conflicts, repeated rebases under H1–H7 linear topic-branch hygiene, and redundant 3-OS CI runs.

The owner directed (2026-11-28) a full audit of all PRs and branches and systematic canonization of governance for `agy`-based automated development.

## Decision

1. **Adopt [[sot:root/agy-automated-development]]** as the normative canon for `agy` automated and parallel development, binding cleanly to [[sot:root/autonomous-operation]] (amended to v2.1.0) without creating a competing product or history SoT.
2. **Replace `claude/*`-only branch reuse with task-scoped branch taxonomy:**
   - Every `agy` work package uses a unique branch `agy/<lane>/<slug>` (lanes: `feat`, `fix`, `refactor`, `docs`, `test`, `verify`, `gov`, `int`, `revert`), deleted on merge (`delete_branch_on_merge: true`).
   - Branches MUST NOT be reused across unrelated PRs.
   - Legacy `claude/*` branches remain recognized during transition and are treated as foreign branches by `agy` sessions (never overwritten or deleted without owner authorization).
3. **Adopt Two-Tier Parallelism to prevent PR chaos:**
   - **Tier A (Hub-and-Spoke Integrator — default):** one `agy` Integrator session decomposes a wave into disjoint path-owned subtasks, dispatches `agy` subagents (`invoke_subagent` with role perspectives), verifies and integrates serially on one branch, runs the fast gate once, and opens **one PR**. Subagents never push, open PRs, or merge.
   - **Tier B (Disjoint Concurrent Lanes — bounded):** multiple independent `agy` sessions may hold open PRs concurrently **only** across disjoint ownership lanes (`L-core`, `L-cli`, `L-remote`, `L-hub`, `L-serve`, `L-agent`, `L-verify`, `L-gov`), capped at **3 open agent PRs** repository-wide, with **shared hot files** prohibited from concurrent modification across open PRs.
4. **Enforce the Zero-Orphan Branch Invariant and Step 0 Sweep:**
   - Pushing an `agy/*` branch MUST be paired with opening a Draft or Ready PR in the same turn.
   - Every `agy` run begins with a mandatory **Step 0 Sweep** (`git ls-remote --heads origin` vs `gh pr list --state open`) to surface unlinked branches, stale/conflicting PRs, and failing checks before starting new work.
5. **Use GitHub as the portable control plane for `agy`:**
   - Branches, Pull Requests (structured Lease Header), Labels (canonical set in `.github/labels.json`), Issues (`needs-owner`), GitHub Actions required checks, and `.spec/plan/` form the runtime-independent coordination bus.
   - CI checks `History` (`playpilot history-check --method merge`) and `Public surface` (`playpilot surface-check`) and the `main-protected` ruleset (`adr:root/20261119220000`) remain **unchanged and binding**.
   - `agy` runs a **portable local pre-push gate** (Rust fmt/clippy/test + Conventional Commits/identity/no-trailer/no-Hangul/no-secret verification) that does not require local `playpilot` installation.
6. **Preserve Autonomy Levels A0 / A1 / A2 unchanged:**
   - A0 (auto-merge), A1 (`needs-human` + recorded measurement, then auto-merge), A2 (owner-only: secrets, rulesets, merge-queue toggle, branch deletion of foreign branches, releases, permission widening).
   - In `agy`, A2 escalation uses `needs-owner` GitHub issues and Korean chat briefings in interactive sessions (replacing `playpilot mail send --to design` when no fleet mailbox exists).
7. **Defer optional GitHub-side settings changes to explicit A2 owner decisions:**
   - Enabling GitHub Merge Queue or toggling `strict_required_status_checks_policy` on `main-protected`, and salvaging or deleting the two legacy orphaned branches (`claude/dazzling-knuth-l8m1nz`, `claude/happy-volta-8xfk6m`), remain A2 owner decisions recorded in [[research:root/20261128034500]].

## Consequences

- Parallel development shifts from uncoordinated N-branch PR collisions to deterministic Integrator-led waves (Tier A) and strictly partitioned lanes (Tier B), reducing rebase thrash and CI waste.
- Orphaned branches become structurally detectable at Step 0 instead of silently drifting behind `main`.
- `agy` sessions can operate end-to-end in remote VMs or desktop workspaces without local `playpilot` or `.claude/settings.json` hooks, while remaining 100% compliant with every server-side check (`History`, `Public surface`, cross-OS CI, `main-protected`).
- Always-on rules `90`, `91`, `92`, `95` and new rule `96` align on the runtime-aware branch scope (`agy/*` and legacy `claude/*`), removing the contradiction between old `claude/*`-only rules and `agy` sessions.

## Alternatives considered

- **Direct push to `main` (dropping PRs):** rejected; violates `main-protected` ruleset, bypasses 3-OS CI + `History` + `Public surface` gates, and increases blast radius under parallel agents.
- **Unbounded 1-PR-per-subagent parallelism:** rejected; causes hot-file rebase loops, CI queue saturation, and orphaned branches (the failure mode observed on 2026-11-25).
- **Mandatory `playpilot` install in every `agy` VM:** rejected; adds external build latency and fleet-secret coupling where GitHub-native coordination + portable local checks suffice.
