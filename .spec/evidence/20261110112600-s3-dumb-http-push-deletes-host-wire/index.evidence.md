---
id: evidence:root/20261110112600
title: S3 dumb HTTP push deletes Claimed (host wire)
status: active
created: 2026-11-10T11:26:00Z
updated: 2026-11-10T11:26:00Z
tags: [s3, r2, dumb-http, webdav, delete, host-wire, claimed, symmetry]
rel:
  depends_on:
    - adr:root/20261110112600
    - adr:root/20261110190000
    - sot:root/arks-remote
    - sot:root/arks-server
    - research:root/20261110105436
    - sot:root/production-residual-register
  evidence: []
files: []
---

# S3 — Dumb HTTP push deletes Claimed (host wire)

## Claimed subset (honest)

| In | Out |
|----|-----|
| WebDAV `DELETE` on `refs/*` after LOCK (arks client) | Stock Apache `mod_dav` / litmus peer |
| Zero-oid push update / CLI `arks push <remote> :<ref>` | Other `src:dst` push refspecs |
| Refuse deleting remote HEAD symref target | Force-with-lease / push-options |
| `update-server-info` after successful delete | Inventing non-WebDAV dumb delete |

Primary: [git-http-push(1)](https://git-scm.com/docs/git-http-push), [RFC 4918](https://datatracker.ietf.org/doc/html/rfc4918) §9.6, stock `t5540` create-and-delete. Unlock: [[adr:root/20261110112600]]. Symmetry ladder: [[research:root/20261110105436]] **S3**.

## Host-wire receipt (Claimed bar)

Same venue class as R2 [[evidence:root/20261110190000]]: in-process `arks-remote` ↔ `arks serve-http --dumb-only` loopback (no Docker stampede; no host GnuPG).

| Check | Outcome |
|-------|---------|
| `dumb_http_push_roundtrip_via_serve_http` (create/FF) | **PASS** (pre-existing R2) |
| Same test: push `refs/heads/topic` then zero-oid delete | **PASS** — topic absent on bare after WebDAV DELETE |
| Same test: refuse delete of `refs/heads/main` (HEAD target) | **PASS** — client Rejected |
| Tip CI `cargo test` workspace (ubuntu/mac/windows) | Carrying when green on land tip |

## Non-claims

- Live forge / G8 latency PASS.
- G19 remasure.
- Treating Apache DAV as Verified peer.
