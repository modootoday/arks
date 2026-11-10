---
id: evidence:root/20261110210500
title: O2 tree:0 partial-clone host wire
status: active
created: 2026-11-10T21:05:00Z
updated: 2026-11-10T21:05:00Z
tags: [o2, r3, tree-0, filter, promisor, host-wire]
rel:
  depends_on:
    - adr:root/20261110210000
    - sot:root/arks-remote
    - research:root/20261110205403
  evidence: []
files: []
---

# O2 — `--filter=tree:0` host wire

## Receipt

| Item | Status |
|------|--------|
| SoT / ADR | [[sot:root/arks-remote]] v0.5.5 + [[adr:root/20261110210000]] |
| Host test | `cargo test -p arks-remote stock_git_http_backend_partial_clone_tree_0` **PASS** |
| Refuse residual | `blob:limit=*` still refuse |
| Clean-room | Expand G39 (`matrix-g39.md` C02); Docker twin when suite run |

Does **not** claim `blob:limit=*`, sparse OID filters, or inventing filter caps.
