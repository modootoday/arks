---
title: Concurrent sessions
description: Multi-agent revision contention — why arks exists and what is claimed
sections:
  - pain
  - response
  - honesty
  - {c: cta, name: next}
---

# Concurrent sessions

This page is the human narrative for the **motive**: many concurrent sessions on one Git revision system.

## What hurts

Agent-governance and multi-agent orchestration routinely open **tens to hundreds of sessions** against shared repositories. Those sessions commit, merge, check out, rebase, and add or move worktrees. Stock Git serializes heavily under that load — revision contention becomes the limiter on parallel work, independent of how clever the orchestrator is.

arks is built for that layer. It is **not** an agent framework, scheduler, or forge UX product.

## What arks does about it

| Layer | Intent (plain language) |
|-------|-------------------------|
| Native Rust engine | Own hot paths in-process so allocator and concurrency policy can evolve without shelling out to `git` for core ops |
| Same `.git` | Stay interchangeable with stock Git for claimed workflows |
| Symmetric CLI + alias | Adopt without retraining every session on a new command name |
| Product leases | Default coordination surface for agent-oriented writers |
| High-concurrency shape | Optional selectable mode; claimed shape includes lease + batch-apply + worktree/ref-shard |
| Shared-index N-way | Separate claimed bar for shared-index contention under clean-room receipts |
| Diverse scenarios | Concurrency catalog work covers many revision races; public copy does **not** say “all races fixed” |

## Honesty (no invented Verified)

Public wording stays conservative:

- **Claimed** high-concurrency / lease / shared-index shapes cite clean-room receipts in the source tree — host-only runs are advisory.
- Live forge functional journeys are claimed only with receipts; harnesses that need secrets may **SKIP** honestly when secrets are absent — never invent PASS.
- Progressive timing bars: full journey allow-list ≤0.8× is measured; aggressive ≤0.5× work is **partial** (PF5 **7/8**; CLONE-LOCAL remains outside that aggressive set).
- Optional index sidecar may sit **beside** `.git`; the Git object store stays filesystem `.git` in Git-compatible mode.
- Deep IDs, matrices, and residual registers live under `.spec/` and `verify/` for maintainers — this site stays thin.

Wall-clock charts (evidence, not slogans): [Performance](../performance/). Scope exclusions: [Limits](../limits/).

## Next

[Alias `git` → `arks`](../alias-git-arks/) [Getting started](../getting-started/) [Performance](../performance/)
