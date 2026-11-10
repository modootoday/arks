---
title: Limits
description: What arks does not claim on the public product surface
nav_order: 70
schema: {faq: auto}
sections:
  - scope
  - storage
  - concurrency
  - {c: faq, name: partial}
  - perf
  - deeper
  - {c: cta, name: next}
---

# Limits

arks aims for Git-compatible CLI use with the same `.git` on disk, aimed at multi-session revision contention. These pages stay short about what is **not** claimed, so you can decide whether to try it.

## Out of product scope here

- Agent frameworks, orchestrators, and IDE products (arks is the **revision** layer those systems call)
- GUI front-ends (`gitk`, `git-gui`, and similar)
- Git LFS and `scalar`
- Forge host APIs (issues, PRs, Checks) — use tools like `gh` for that layer
- Shipping arks as a Docker-based end-user runtime — the product is the **native CLI**

## Storage

- In Git-compatible mode, the **object store is filesystem `.git`**.
- An optional index sidecar may sit **beside** `.git` for product queries. arks does not advertise replacing Git objects with a different object store.

## Concurrency honesty

- Product leases and claimed high-concurrency / shared-index shapes are **not** a warranty that every multi-writer race is solved.
- Live forge throughput, Checks, and merge-queue behavior are not claimed as product SLOs.
- See [Concurrent sessions](../concurrent-sessions/) for the motive without gate folklore.

## Partial or refused features

### Partial clone / dumb push?

`--depth` shallow clones may be used on pack transports (smart HTTP, SSH, `git://`); filter / `blob:none` / promisor-style partial clone is not claimed. Dumb HTTP **push** uses a WebDAV write subset (LOCK/MKCOL/PUT/MOVE) when receive-pack discovery is dumb; deletes and stock Apache DAV interop are not claimed.

### Commit and tag signing?

SSH create/verify for commit `gpgsig` may be used; OpenPGP, X.509, and `tag -s` are not claimed.

### Credentials?

Helper / askpass / TTY / first-party OS wallets (`wincred` / `libsecret` / `osxkeychain`) may be used; forge OAuth is not claimed.

### Alias coverage?

Unimplemented commands fail closed (non-zero, no repo mutation). See [Alias `git` → `arks`](../alias-git-arks/).

### Aggressive ≤0.5× journeys?

Latest sole G19 remeasure: **all 19** allow-list journeys ≤0.5× (**Claimed 8/8**, including Clone HTTP). See [Performance](../performance/).


### Live harnesses?

May SKIP without secrets; never invent PASS.

## Performance wording

This site does **not** claim “faster than Git” (or leaner / higher-throughput) as marketing. Comparative clean-room measurements belong next to their receipts — see [Performance](../performance/).

## Where deeper design lives

Normative specs and verification harnesses live in the source tree under `.spec/` and `verify/`. They are maintainer material; this site stays a human product narrative.

Source: [modootoday/arks](https://github.com/modootoday/arks)

## Next

[Getting started](../getting-started/) [Concurrent sessions](../concurrent-sessions/) [Contribute](../contribute/)
