---
layout: doc
toc: false
title: arks for Git
short_title: arks
description: Agent Revision Kinetic System — Git-compatible revision CLI for multi-session contention.

hero:
  c: split
  visual: code
  chrome: terminal
  side: end
  ratio: "5:7"
sections:
  - {c: features, name: why, cols: 3}
  - {c: chart, name: timings, type: bar, data: m18-highlight-ms}
  - {c: cta, name: next, bleed: true}
---

# arks for Git

**arks** — **Agent Revision Kinetic System** — is a Git-compatible revision CLI you build and run natively. It exists because **multi-agent and multi-session orchestration** put tens to hundreds of concurrent writers on the same repositories — and Git revision contention becomes the limiter.

```bash
alias git=arks
arks status
```

arks keeps the **same `.git` on disk**, a **symmetric CLI**, and an adopt path of `alias git=arks` (optional `alias gh=arks`). Self-host via **`arks serve`**; forge client leaves are flat (`arks pr`, …). It is the revision layer for those environments — **not** an agent framework.

[Getting started](getting-started/) [Completeness](completion/) [Self-host](self-host/) [Concurrent sessions](concurrent-sessions/) [Source](https://github.com/modootoday/arks)

## Why try arks

- **Built for contention** — Many concurrent sessions committing, checking out, merging, and moving worktrees. Product leases and claimed high-concurrency shapes target that load without abandoning Git interchange. `status:available`
- **Same repositories** — Everyday init, add, commit, branch, fetch, and push target ordinary Git object, ref, and index formats. `status:available`
- **Adopt by alias** — Familiar argv where claimed. Alias `git` / `gh` to `arks` (or use a PATH shim) while you evaluate. Native binary — Docker is for maintainer clean-room verification only. `status:partial`

## Clean-room timings (sample)

Selected medians from a Docker clean-room receipt (stock Git 2.39.5 vs release `arks`). Read them as wall-clock evidence on shared journeys — including ones that show up under parallel workflows — **not** as a “faster than Git” slogan.

*Median wall-clock ms — selected journeys (clean-room G19 remeasure at tip `c1b0b24`)*

Full table, ratio honesty (M17 **19/19** ≤0.8×; tip PF5 **Claimed(partial)** with honest `aggressive_na`), and evidence links: [Performance](performance/). Feature Claimed vs advisory vs Impossible: [Git vs arks](git-vs-arks/). Completion milestone (design + shell, honest Partial): [Completeness](completion/).


## Next

[Build and try it](getting-started/) [Read the contention story](concurrent-sessions/) [See completeness](completion/) [See the limits](limits/)
