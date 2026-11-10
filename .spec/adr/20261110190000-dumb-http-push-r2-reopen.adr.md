---
id: adr:root/20261110190000
title: Reopen R2 — dumb HTTP push (WebDAV write subset)
status: active
created: 2026-11-10T19:00:00Z
updated: 2026-11-10T19:00:00Z
tags: [r2, dumb-http, webdav, product-completion, l3, arks-remote, arks-server]
rel:
  depends_on:
    - sot:root/arks-remote
    - sot:root/arks-server
    - sot:root/production-residual-register
    - research:root/20261110095142
    - research:root/20261108071916
  supersedes: []
  evidence: [evidence:root/20261110190000]
---

# Reopen R2 — dumb HTTP push (WebDAV write subset)

## Context

Residual **R2** / ladder **L3** left dumb HTTP **push** explicit **no** while dumb fetch/clone and smart receive-pack push were Claimed. Product-completion wants the client/server asymmetry closed without inventing pack-protocol.

## Decision

Amend [[sot:root/arks-remote]] and [[sot:root/arks-server]] to claim an additive **dumb HTTP push** subset:

1. **Trigger:** when `GET …/info/refs?service=git-receive-pack` returns a dumb body (non-`application/x-git-*` / no smart service pkt-line), the client MUST fall back to WebDAV write instead of refusing ([gitprotocol-http](https://git-scm.com/docs/gitprotocol-http) smart→dumb fallback pattern; push write shape from [git-http-push(1)](https://git-scm.com/docs/git-http-push)).
2. **Wire methods (RFC 4918):** `PROPFIND` (lock support), `LOCK` / `UNLOCK`, `MKCOL`, `PUT`, optional `MOVE` for loose objects and ref resources.
3. **Layout:** loose object URLs and zlib bodies per [gitprotocol-http](https://git-scm.com/docs/gitprotocol-http) + [gitrepository-layout](https://git-scm.com/docs/gitrepository-layout); ref files hold `oid LF`.
4. **Fast-forward:** without `--force`, refuse non-ancestor updates ([git-http-push(1)](https://git-scm.com/docs/git-http-push)).
5. **Server:** additive WebDAV write beside smart HTTP; after successful ref PUT, refresh dumb ads via `update-server-info` ([git-update-server-info](https://git-scm.com/docs/git-update-server-info)). **Dumb-only as sole server remains forbidden.**

## Non-claims

- Stock Apache `mod_dav` / litmus full suite as a verified peer.
- `git http-push` `-d`/`-D` delete path; force-with-lease; push-options.
- Treating the obsolete howto [setup-git-server-over-http](https://www.kernel.org/pub/software/scm/git/docs/howto/setup-git-server-over-http.html) as the recommend path (howto itself points to smart `git-http-backend`).
- Inventing a private non-WebDAV “PUT pack” dumb push.

## Consequences

- Host wire: arks-remote ↔ `arks serve-http` with `dumb_only` (or equivalent dumb receive-pack discovery) proves create/FF push + object GET after `update-server-info`.
- Residual **R2** → **Claimed (host wire)** with evidence; ladder L3 Done; next pointer L4.
- Clean-room matrix twin optional later (not required for this Claimed bar).
