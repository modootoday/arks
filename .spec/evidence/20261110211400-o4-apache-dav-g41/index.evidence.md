---
id: evidence:root/20261110211400
title: O4 Apache mod_dav G41 clean-room PASS
status: active
created: 2026-11-10T21:14:00Z
updated: 2026-11-10T21:14:00Z
tags: [o4, apache-dav, g41, dumb-http, webdav, clean-room]
rel:
  depends_on:
    - adr:root/20261110210400
    - sot:root/arks-remote
    - sot:root/arks-server
    - sot:root/interop-test-matrix
    - research:root/20261110205403
  evidence: []
files:
  - summary.txt
---

# O4 — Apache `mod_dav` dumb HTTP interop (G41 PASS)

## Receipt

| Item | Status |
|------|--------|
| Compose | `g41-seed` → `g41-dav` (`httpd:2.4-bookworm`) → `g41` |
| Matrix | `verify/matrices/matrix-g41.md` |
| Test | `g41_arks_push_create_ff_and_delete_vs_apache_dav` **PASS** |
| Summary | `verify/gates/g41/out/summary.txt` → `G41 PASS` |
| Tip exercised | ≥ `0234159` |

## Non-claims

Full Apache litmus; force-with-lease on dumb; inventing non-WebDAV delete; `proc-receive`.
