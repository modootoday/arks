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
- Embedding full GUI front-ends (`gitk`, `git-gui`) inside the CLI binary — **FS7** external handoff Claimed (refuse+reason); not an embed
- Git LFS — **trimmed** from arks product CLI/server; use external `git-lfs` (stock git has no LFS builtin)
- Forge host APIs (issues, PRs, Checks) — not part of git-alias DoD; design home is [`arks hub`](../arks-hub/) on the **same** `arks` binary (gh-shaped command group, native API — design intent, not Claimed; **no** separate hub release)
- Claiming WAN “secure by default” self-host without TLS/ops — see [Self-host](../self-host/) (`arks serve` default HTTP+SSH; ops owns firewall/DNS/TLS; in-process SSH/TLS not Claimed)
- Shipping arks as a Docker-based end-user runtime — the product is the **native CLI**
- Absolute SHA-1 collision **immunity**, full cloud-sync safety on `.git`, embedded forge OAuth UX, and crates.io timing (**L9**) — documented Impossible / advisory / owner-policy (see [Git vs arks](../git-vs-arks/))

## Storage

- In Git-compatible mode, the **object store is filesystem `.git`**.
- An optional index sidecar may sit **beside** `.git` for product queries. arks does not advertise replacing Git objects with a different object store.

## Concurrency honesty

- Product leases and claimed high-concurrency / shared-index shapes are **not** a warranty that every multi-writer race is solved.
- Live forge throughput, Checks, and merge-queue behavior are not claimed as product SLOs.
- See [Concurrent sessions](../concurrent-sessions/) for the motive without gate folklore.

## Partial or refused features

### Partial clone / dumb push?

`--depth` shallow clones may be used on pack transports (smart HTTP, SSH, `git://`). Partial-clone `--filter=blob:none`, `--filter=tree:0`, FS4 `tree:<n>` / `blob:limit`, G6 `object:type` / `combine:`, and `sparse:oid=<hex>` (promisor + `extensions.partialClone`) are claimed on those pack transports; `blob:limit=*` and other unlisted exotic specs remain refused. Dumb HTTP **push** uses a WebDAV write subset (LOCK/MKCOL/PUT/MOVE/DELETE) when receive-pack discovery is dumb, including empty-refspec deletes (`push :<ref>`), with stock Apache `mod_dav` interop **Claimed (G41)**. Smart-pack `push --force-with-lease` and `--push-option` are claimed; dumb HTTP lease is a **CAS-over-DAV** subset (LOCK + re-GET tip) — `--push-option` stays smart-only. Mixed sha1↔sha256 remotes refuse fail-closed (no OID rewrite).

### Commit and tag signing?

SSH create/verify for commit `gpgsig` may be used (`gpg.format=ssh`). OpenPGP and X.509 commit signing (`gpg.format=openpgp` / `x509`) plus `tag -s` / `verify-tag` are claimed via spawned `gpg` / `gpgsm` (clean-room **G38**); forge signature UX is not claimed.

### Credentials?

Helper / askpass / TTY / first-party OS wallets (`wincred` / `libsecret` / `osxkeychain`) may be used; forge OAuth is not claimed.

### Interactive / patch?

**FS1/FS3 Claimed:** hunk-level `add -p` (TTY or `ARKS_ADD_PATCH_ANSWERS`), `add -i` menu, `add -e`, `stash -p`, and `rebase -i` via sequence/editor fallback. **FS5/NET-006:** `clone --bundle-uri` accepts local paths and `http(s)://` with concurrent-lock and Content-Length honesty. **FS6:** checkout honors stock `filter.*.smudge` only (in-process LFS productization **trimmed**). **OBJ-004:** `fsck` refuses OID mismatches; SHA-1 uses collision **detect+refuse** (`sha1collisiondetection`); absolute collision immunity remains Impossible; sha256 porcelain Claimed. **FS-007 / WIN-003 / host-env:** product CLI **trimmed** (not git). See [Git vs arks](../git-vs-arks/).

### Alias coverage?

Unimplemented commands fail closed (non-zero, no repo mutation). See [Alias `git` → `arks`](../alias-git-arks/).

### Aggressive ≤0.5× journeys?

M17 ≤0.8× remains **19/19**. Aggressive ≤0.5×: historical **Claimed 8/8** at `79fc9bf`; tip is **Claimed(partial)** (18 ≤0.5× incl. REVERT; honest `aggressive_na` for alias process-tax — no thrash remasure). Absolute SHA-1 collision **immunity** remains Impossible (detect+refuse is the Claimed bar — do not market “immune”). See [Performance](../performance/).


### Live harnesses?

May SKIP without secrets; never invent PASS. Maintainer dry-run / Environment wiring notes: [Forge scale harness](../forge-scale/).

## Performance wording

This site does **not** claim “faster than Git” (or leaner / higher-throughput) as marketing. Comparative clean-room measurements belong next to their receipts — see [Performance](../performance/).

## Where deeper design lives

Normative specs and verification harnesses live in the source tree under `.spec/` and `verify/`. They are maintainer material; this site stays a human product narrative.

Source: [modootoday/arks](https://github.com/modootoday/arks)

## Next

[Self-host](../self-host/) [Git vs arks](../git-vs-arks/) [Getting started](../getting-started/) [Concurrent sessions](../concurrent-sessions/) [Contribute](../contribute/)
