---
title: Overview
description: What arks is — Agent Revision Kinetic System for Git-compatible multi-session work
nav_order: 10
sections:
  - motive
  - what
  - priorities
  - naming
  - {c: cta, name: next}
---

# Overview

**arks** (tagline: **arks for Git**) is a **native Rust** Git-compatible revision CLI and infra, built **for** agent / multi-session environments that thrash Git’s revision layer, without being an agent product itself.

You install and run the `arks` binary on your machine. Docker is used by maintainers for clean-room verification — it is not the product runtime.

## Motive

In agent-governance-based orchestration, **tens to hundreds of concurrent sessions** share repositories. Commit, worktree, merge, checkout, and related revision ops contend. That contention caps how much parallel work the swarm can finish.

arks improves that **revision system**: native engine for performance inside a standards-first envelope, diverse contention scenarios under lease / concurrency design, and a **fully symmetric CLI** so teams adopt immediately via **alias**.

## What it is (and is not)

| Is | Is not |
|----|--------|
| Git-compatible CLI + claimed server role | An agent framework, IDE, or orchestrator |
| Same filesystem `.git` in Git-compatible mode | A parallel proprietary object store for day-to-day Git work |
| Alias-ready argv where claimed | A warranty that every Git flag ships today |
| Product leases + optional high-concurrency modes | “All races fixed” marketing |

> [!IMPORTANT]
> Public copy separates **Claimed** (with clean-room receipts) from **Verified** host folklore. This site does not invent PASS results or blanket “faster than Git” slogans.

## Priorities

| Priority | Meaning |
|----------|---------|
| Compatibility first | Honor documented Git object, ref, index, pack, protocol, and CLI semantics |
| Performance second | Faster / more parallel paths only inside that envelope — numbers live on [Performance](../performance/) with receipts |

## Naming

- **Product / crate / CLI binary**: `arks`
- **Tagline**: **arks for Git**
- **Working expansion** (once): **Agent Revision Kinetic System**
- Drop-in usage is via alias or PATH shim — the short name does not contain a literal `git` substring

## Next

[Getting started](../getting-started/) [Concurrent sessions](../concurrent-sessions/) [Alias `git` → `arks`](../alias-git-arks/) [Limits](../limits/)
