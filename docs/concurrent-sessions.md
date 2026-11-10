---
title: Concurrent sessions
description: Multi-agent revision contention — why arks exists and what is claimed
nav_order: 20
sections:
  - pain
  - {c: timeline, name: response}
  - {c: accordion, name: honesty}
  - {c: cta, name: next}
---

# Concurrent sessions

This page is the human narrative for the **motive**: many concurrent sessions on one Git revision system.

## What hurts

Agent-governance and multi-agent orchestration routinely open **tens to hundreds of sessions** against shared repositories. Those sessions commit, merge, check out, rebase, and add or move worktrees. Stock Git serializes heavily under that load — revision contention becomes the limiter on parallel work, independent of how clever the orchestrator is.

arks is built for that layer. It is **not** an agent framework, scheduler, or forge UX product.

## What arks does about it

### Native Rust engine

Own hot paths in-process so allocator and concurrency policy can evolve without shelling out to `git` for core ops.

### Same `.git`

Stay interchangeable with stock Git for claimed workflows — ordinary filesystem object store in Git-compatible mode.

### Symmetric CLI + alias

Adopt without retraining every session on a new command name — see [Alias `git` → `arks`](../alias-git-arks/).

### Product leases

Default coordination surface for agent-oriented writers.

### High-concurrency shape

Optional selectable mode; claimed shape includes lease + batch-apply + worktree/ref-shard.

### Shared-index N-way

Separate claimed bar for shared-index contention under clean-room receipts — not conflated with the high-concurrency lease shape.

## Honesty (no invented Verified)

### Claimed vs host-only

**Claimed** high-concurrency / lease / shared-index shapes cite clean-room receipts in the source tree — host-only runs are advisory.

### Live forge journeys

Live forge functional journeys are claimed only with receipts; harnesses that need secrets may **SKIP** honestly when secrets are absent — never invent PASS.

### Timing bars

Full journey allow-list ≤0.8× is measured (**19/19** on the latest sole G19 remasure). Aggressive ≤0.5× is **Claimed(partial)** (REVERT Claimed; alias process-tax stays honest `aggressive_na` — no thrash remasure). Charts: [Performance](../performance/).


### Index sidecar

Optional index sidecar may sit **beside** `.git`; the Git object store stays filesystem `.git` in Git-compatible mode.

### Where deep IDs live

Deep IDs, matrices, and residual registers live under `.spec/` and `verify/` for maintainers — this site stays thin. Scope exclusions: [Limits](../limits/).

## Next

[Alias `git` → `arks`](../alias-git-arks/) [Getting started](../getting-started/) [Performance](../performance/)
