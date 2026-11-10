---
id: adr:root/20261110212500
title: FS2 — proc-receive advertise + divert (positive prefixes)
status: accepted
created: 2026-11-10T21:25:00Z
updated: 2026-11-10T21:25:00Z
tags: [fs2, proc-receive, receive-pack, full-symmetry]
rel:
  depends_on:
    - research:root/20261110212414
    - adr:root/20261110210400
    - sot:root/arks-server
---

# FS2 — proc-receive advertise + divert

## Context

O4 Claimed push-options → hooks and explicitly kept `proc-receive` unadvertised. Full-symmetry DoD ([[research:root/20261110212414]]) reopens real git protocol features previously left as casual N/A.

## Decision

1. Advertise `proc-receive` **only** when `$GIT_DIR/hooks/proc-receive` is runnable.
2. Divert update commands whose refnames match positive `receive.procReceiveRefs` prefixes to the hook via pkt-line **version=0** protocol.
3. Honor hook results: `ok` / `ng <msg>` plus optional `option refname|old-oid|new-oid` rewrite before applying the target ref update.
4. **Non-claim:** HOOK-008 race freedom (FS8); negative `!` prefix forms.

## Consequences

- [[sot:root/arks-server]] flips `proc-receive` from N/A → Claimed (FS2).
- Public comparison / residual currency updated with evidence; no invent PASS for race harnesses.
