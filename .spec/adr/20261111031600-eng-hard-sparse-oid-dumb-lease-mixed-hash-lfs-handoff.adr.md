---
id: adr:root/20261111031600
title: Eng-hard reopen — sparse:oid, dumb force-with-lease CAS, mixed-hash refuse, LFS handoff
status: accepted
created: 2026-11-10T18:16:00Z
updated: 2026-11-10T18:16:00Z
tags: [sparse-oid, force-with-lease, dumb-http, mixed-hash, lfs, tus, eng-hard]
rel:
  depends_on:
    - research:root/20261111030858
    - research:root/20261111022345
    - sot:root/arks-remote
    - sot:root/pf5-journey-seed-amortization
    - adr:root/20261110214800
    - adr:root/20261110210400
    - plan:root/20261111024440
---

# Eng-hard reopen — sparse:oid, dumb force-with-lease CAS, mixed-hash refuse, LFS handoff

## Context

Survey [[research:root/20261111030858]] ranked Eng-hard residuals after G-ladder Done: Pf5 J-ALIAS PATH/`exec`, `sparse:oid`, dumb HTTP `force-with-lease`, LFS tus/S3 handoff, mixed sha1↔sha256 honesty. Operators cannot invent g8-live PASS; L9 stays owner-closed.

## Decision

| Surface | Claimed slice | Explicit non-claim |
|---------|---------------|--------------------|
| **Pf5 J-ALIAS** | Product cut: hard-link (else hard-copy) `git` shim + absolute `exec` **without** PATH rewrite → **exactly one** sole Docker G19; ≤0.5× Claimed **or** keep `aggressive_na` | In-process multi-command amortize; G19 thrash |
| **`sparse:oid=<hex>`** | Client parse/gate + wire `filter sparse:oid=<40\|64 hex>` + promisor/backfill path already used for Claimed filters; stock peers that advertise `filter` omit per Git | Blob-ish beyond hex; arks-server as sole filter product; multi-`--filter` |
| **Dumb `--force-with-lease`** | CAS-over-DAV: advertise tip check + exclusive LOCK + re-GET tip before PUT; lease match may non-FF | Invented lease tokens; `--push-option` on dumb |
| **Mixed objectFormat** | Homogeneous sha1 or sha256 ads when they match local `extensions.objectFormat`; refuse mismatch with clear reason | Cross-hash OID rewrite / mixed remotes product |
| **LFS tus / S3** | Refuse non-`basic` batch transfer; `locks/verify` pre-push; exotic argv (`tus`/`transfer`/…) hand off to `git-lfs` | In-process tus adapter; multi-tenant S3 product |

## Consequences

Amend [[sot:root/arks-remote]] v0.5.6 rules 7c / 15 / 18. Update residual **FW-LFS** / filter / ALIAS / OBJ-004 reopen notes and public `docs/limits.md` / `docs/git-vs-arks.md`. File evidence for host tests + **one** sole G19 for ALIAS. Do not invent live G8 or L9 PASS.
