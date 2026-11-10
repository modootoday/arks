---
layout: home
toc: false
title: arks for Git
description: Agent Revision Kinetic System — Git-compatible revision CLI for multi-session contention.
sections:
  - {c: features, name: why, cols: 3}
  - sample
  - timings
  - {c: cta, name: next}
---

# arks for Git

**arks** — **Agent Revision Kinetic System** — is a Git-compatible revision CLI you build and run natively. It exists because **agent-governance and multi-session orchestration** put tens to hundreds of concurrent writers on the same repositories — and Git revision contention becomes the limiter.

arks keeps the **same `.git` on disk**, a **symmetric CLI**, and an adopt path of `alias git=arks`. It is the revision layer for those environments — **not** an agent framework.

[Getting started](getting-started/) [Concurrent sessions](concurrent-sessions/) [Alias `git` → `arks`](alias-git-arks/) [Source](https://github.com/modootoday/arks)

## Why try arks

- **Built for contention** — Many concurrent sessions committing, checking out, merging, and moving worktrees. Product leases and claimed high-concurrency shapes target that load without abandoning Git interchange.
- **Same repositories** — Everyday init, add, commit, branch, fetch, and push target ordinary Git object, ref, and index formats.
- **Adopt by alias** — Familiar argv where claimed. Alias `git` to `arks` (or use a PATH shim) while you evaluate. Native binary — Docker is for maintainer clean-room verification only.

## Sample session

```bash
cargo build -p arks --release
arks init demo && cd demo
echo "hello" > README.md
arks add README.md
arks commit -m "Initial commit"
arks status
```

Same directory, stock Git still works — the `.git` directory is ordinary Git data.

## Clean-room timings (sample)

These medians come from a Docker **clean-room verification** run (stock Git 2.39.5 vs release `arks`). Read them as wall-clock evidence on shared journeys — including ones that show up under parallel workflows — not as a “faster than Git” slogan. Full table, PF5 partial honesty, and evidence links: [Performance](performance/).

`chart:bar`

| Journey | Git | arks |
|---------|----:|-----:|
| Tag | 177 | 64 |
| Diff | 199 | 74 |
| Submodule | 92 | 16 |
| Clone path | 37 | 12 |
| Repack | 29 | 10 |
| Local | 60 | 40 |

*Median wall-clock ms — selected journeys (clean-room M18 / G19 receipt)*

## Next

[Build and try it](getting-started/) [Read the contention story](concurrent-sessions/) [See the limits](limits/)
