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

`--depth` shallow clones may be used on pack transports (smart HTTP, SSH, `git://`). Partial-clone `--filter=blob:none` and `--filter=tree:0` (promisor + `extensions.partialClone`) are claimed on those pack transports; other filter-specs remain refused. Dumb HTTP **push** uses a WebDAV write subset (LOCK/MKCOL/PUT/MOVE/DELETE) when receive-pack discovery is dumb, including empty-refspec deletes (`push :<ref>`), with stock Apache `mod_dav` interop **Claimed (G41)**. Smart-pack `push --force-with-lease` and `--push-option` are claimed; lease on dumb HTTP is not.

### Commit and tag signing?

SSH create/verify for commit `gpgsig` may be used (`gpg.format=ssh`). OpenPGP and X.509 commit signing (`gpg.format=openpgp` / `x509`) plus `tag -s` / `verify-tag` are claimed via spawned `gpg` / `gpgsm` (clean-room **G38**); forge signature UX is not claimed.

### Credentials?

Helper / askpass / TTY / first-party OS wallets (`wincred` / `libsecret` / `osxkeychain`) may be used; forge OAuth is not claimed.

### Interactive TUI?

Full curses/TTY TUI menus stay **out of claim**. Scripted modes are Claimed (O3): `rebase -i` with `GIT_SEQUENCE_EDITOR`/`ARKS_SEQUENCE_EDITOR`, editor `commit` via `GIT_EDITOR`/`ARKS_EDITOR`/`-F`, and `add -p` via `ARKS_ADD_PATCH_ANSWERS` or non-TTY stdin (file-granularity). See [Git vs arks](../git-vs-arks/).

### Alias coverage?

Unimplemented commands fail closed (non-zero, no repo mutation). See [Alias `git` → `arks`](../alias-git-arks/).

### Aggressive ≤0.5× journeys?

Latest sole G19 remeasure: **all 19** allow-list journeys ≤0.5× (**Claimed 8/8**, including Clone HTTP). See [Performance](../performance/).


### Live harnesses?

May SKIP without secrets; never invent PASS. Maintainer dry-run / Environment wiring notes: [Forge scale harness](../forge-scale/).

## Performance wording

This site does **not** claim “faster than Git” (or leaner / higher-throughput) as marketing. Comparative clean-room measurements belong next to their receipts — see [Performance](../performance/).

## Where deeper design lives

Normative specs and verification harnesses live in the source tree under `.spec/` and `verify/`. They are maintainer material; this site stays a human product narrative.

Source: [modootoday/arks](https://github.com/modootoday/arks)

## Next

[Git vs arks](../git-vs-arks/) [Getting started](../getting-started/) [Concurrent sessions](../concurrent-sessions/) [Contribute](../contribute/)
