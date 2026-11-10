---
id: evidence:root/20261110210700
title: O4 force-with-lease + push-options host wire
status: active
created: 2026-11-10T21:07:00Z
updated: 2026-11-10T21:07:00Z
tags: [o4, force-with-lease, push-options, smart-http]
rel:
  depends_on:
    - adr:root/20261110210400
    - sot:root/arks-remote
    - sot:root/arks-server
    - research:root/20261110205403
  evidence: []
files: []
---

# O4 — force-with-lease + push-options host wire

## Receipt

| Item | Status |
|------|--------|
| Host test | `cargo test -p arks-remote smart_http_force_with_lease_and_push_options` **PASS** |
| Stale lease | Rejected fail-closed |
| Matching lease + `-o ci.skip` | Push accepted; server advertises `push-options` |
| Dumb HTTP lease | Still N/A (refuse) |
| Apache DAV | Matrix `matrix-g41.md` structural; Claimed only with G41 receipt |
| `proc-receive` / HOOK-008 | Still N/A |
