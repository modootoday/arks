---
id: adr:root/20261110210400
title: O4 reopen — force-with-lease, push-options, Apache DAV
status: active
created: 2026-11-10T21:04:00Z
updated: 2026-11-10T21:04:00Z
tags: [o4, force-with-lease, push-options, apache-dav, dumb-http, protocol-narrow]
rel:
  depends_on:
    - research:root/20261110205403
    - adr:root/20261110112600
    - adr:root/20261110190000
    - sot:root/arks-remote
    - sot:root/arks-server
    - sot:root/production-residual-register
  supersedes: []
  evidence: [evidence:root/20261110210700, evidence:root/20261110211400]
---

# O4 — Protocol narrows: force-with-lease, push-options, Apache DAV

## Context

S3 inventory left force-with-lease, push-options / `proc-receive`, and stock Apache DAV as honest defer ([[adr:root/20261110112600]]). Optional ladder [[research:root/20261110205403]] **O4** reopens SoTable protocol rows.

## Decision

1. **force-with-lease (client):** Claim `arks push --force-with-lease` and `--force-with-lease=<ref>:<expect>` on smart pack transports. Wire uses the expected OID as the receive-pack command `old` value; client MAY fail closed early when the advertisement tip ≠ expect. Dumb HTTP remains FF-only (lease force N/A on WebDAV until separately claimed).
2. **push-options (client + server):** Advertise `push-options` on receive-pack; accept `push-option <data>` pkt-lines after the command flush; expose `GIT_PUSH_OPTION_COUNT` / `GIT_PUSH_OPTION_<n>` to honor-list hooks. **Do not** claim `proc-receive` rewrite / HOOK-008 race freedom.
3. **Apache DAV dumb interop:** Claim dumb HTTP push create/FF/delete against stock Apache `mod_dav` (+ `mod_cgi`/`Alias` bare repo) in clean-room when Docker-SoTable; host advisory MAY use a loopback fixture. Retain non-claim for litmus suite completeness and DAV locking edge dialects beyond the git-http-push subset already Claimed.

## Non-claims

| Narrow | Disposition |
|--------|-------------|
| `proc-receive` capability / rewrite | Still N/A |
| HOOK-008 concurrent push-options races | Still N/A (catalog) |
| force-with-lease on dumb WebDAV | Still N/A |
| Full Apache litmus / every DAV class | Still N/A |

## Consequences

- Amend [[sot:root/arks-remote]] + [[sot:root/arks-server]]; flip residual / comparison rows only with evidence.
- Next **O_final** currency.
