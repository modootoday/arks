---
id: adr:root/20261110112600
title: Reopen dumb HTTP push deletes (S3)
status: active
created: 2026-11-10T11:26:00Z
updated: 2026-11-10T11:26:00Z
tags: [s3, r2, dumb-http, webdav, delete, protocol-narrow, arks-remote, arks-server]
rel:
  depends_on:
    - adr:root/20261110190000
    - sot:root/arks-remote
    - sot:root/arks-server
    - sot:root/production-residual-register
    - research:root/20261110105436
  supersedes: []
  evidence: [evidence:root/20261110112600]
---

# Reopen dumb HTTP push deletes (S3)

## Context

[[adr:root/20261110190000]] Claimed dumb HTTP **create/FF** WebDAV push (R2 / L3) but left `git http-push` **`-d`/`-D`** and empty-refspec deletes (`push :ref`) as explicit non-claims. Symmetry ladder [[research:root/20261110105436]] **S3** reopens SoTable protocol narrows; WebDAV `DELETE` on ref paths is the primary implementable row ([git-http-push(1)](https://git-scm.com/docs/git-http-push), [RFC 4918](https://datatracker.ietf.org/doc/html/rfc4918) §9.6; stock `t5540` create-and-delete).

## Decision

1. **Client ([[sot:root/arks-remote]]):** when a push update has zero `new_oid`, on the dumb WebDAV path MUST `LOCK` the ref URL, send `DELETE`, then `UNLOCK` (or accept peer success without lock when unlocked). MUST refuse deleting the remote HEAD symref target. Mixed create+delete in one push MAY upload objects for non-delete tips then apply deletes.
2. **Server ([[sot:root/arks-server]]):** accept `DELETE` under `refs/` (additive beside PROPFIND/LOCK/UNLOCK/MKCOL/PUT/MOVE). After successful delete, refresh `update-server-info`. MUST refuse deleting the current HEAD target.
3. **CLI:** claim empty-source delete refspec `:<ref>` on `arks push` (fully-qualified or `heads/` shorthand). Other `src:dst` refspecs remain unclaimed.
4. **Evidence:** host-wire loopback vs `arks serve-http --dumb-only` (same bar class as R2 [[evidence:root/20261110190000]]); no host GnuPG / cargo stampede required.

## Non-claims / honest defer (still S3 inventory)

| Narrow | Disposition |
|--------|-------------|
| Stock Apache `mod_dav` / litmus peer | Still N/A (R2 non-claim retained) |
| Force-with-lease / push-options / `proc-receive` | Still N/A |
| `http-push -d` porcelain flag name | N/A — claim empty-refspec `:` delete (git push shape) |
| Inventing non-WebDAV dumb delete | Forbidden |

## Consequences

- Amend R2 non-claim bullet for deletes; residual pointer flips to **Claimed (S3)**.
- Symmetry **S3** Done for this row; next **S4** (interactive / patch flows) or remaining honest-defer rows.
- **L9** stays closed.
