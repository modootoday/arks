---
id: evidence:root/20261110190000
title: R2 dumb HTTP push Claimed (host wire)
status: active
created: 2026-11-10T19:00:00Z
updated: 2026-11-10T19:00:00Z
tags: [r2, l3, dumb-http, webdav, host-wire, claimed, product-completion]
rel:
  depends_on:
    - adr:root/20261110190000
    - sot:root/arks-remote
    - sot:root/arks-server
    - sot:root/production-residual-register
    - research:root/20261110095142
  evidence: []
files: []
---

# R2 — Dumb HTTP push Claimed (host wire)

## Claimed subset (honest)

| In | Out |
|----|-----|
| Smart→dumb fallback on receive-pack discovery | Invented private non-WebDAV push |
| WebDAV PROPFIND / LOCK / UNLOCK / MKCOL / PUT / MOVE | Stock Apache `mod_dav` litmus as verified peer |
| FF create/update of `refs/heads/*`; zlib loose objects | `-d`/`-D` delete; force-with-lease; push-options |
| `arks serve-http` additive DAV write + `update-server-info` | Dumb-only as sole claimed server |

Primary: [git-http-push(1)](https://git-scm.com/docs/git-http-push), [RFC 4918](https://datatracker.ietf.org/doc/html/rfc4918), [gitprotocol-http](https://git-scm.com/docs/gitprotocol-http) object layout, [gitrepository-layout](https://git-scm.com/docs/gitrepository-layout), [git-update-server-info](https://git-scm.com/docs/git-update-server-info). Unlock: [[adr:root/20261110190000]].

## Host advisory (Claimed bar for this reopen)

| Check | Outcome |
|-------|---------|
| `cargo test -p arks-server --lib` | PASS |
| `cargo test -p arks-remote --lib dumb_http_push_roundtrip_via_serve_http` | PASS — clone + WebDAV push vs `spawn_loopback_with_opts(dumb_only=true)` |
| `cargo test -p arks-remote --lib` | PASS (35 tests) |

Host green alone is not Docker Verified. Optional clean-room twin may follow; not required for R2 Claimed (host wire) per residual reopen path.

## Non-claims

- Live forge / secrets-gated gates.
- Stock `git http-push` against Apache as a carrying peer.
- G19 remasure.
