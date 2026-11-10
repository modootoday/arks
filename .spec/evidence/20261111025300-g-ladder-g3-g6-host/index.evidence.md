---
id: evidence:root/20261111025300
title: G3–G6 host evidence — bundle-uri, LFS locks, forge recipe, exotic filters
status: active
created: 2026-11-10T17:53:00Z
updated: 2026-11-10T17:53:00Z
tags: [g3, g4, g5, g6, bundle-uri, lfs-locks, forge, filters, g-ladder]
verify: host
rel:
  depends_on:
    - plan:root/20261111024440
    - sot:root/arks-server
    - sot:root/arks-remote
---

# G3–G6 host evidence

Host-advisory unit tests (clean-room e2e for new Claimed surfaces follows G_final / CI).

| Goal | Slice | Proof |
|------|-------|-------|
| **G3** | Server protocol-v2 `bundle-uri` advertise + command | `cargo test -p arks-server --lib` → `v2_advertises_and_serves_bundle_uri` **PASS** |
| **G4** | LFS locks create/list/unlock on `serve-http` | `lfs_locks_create_list_unlock` **PASS**; CLI `arks lfs lock|locks|unlock` |
| **G5** | Forge recipe deepen | `arks forge recipe` text (merge-queue/release/checks recipes); still Claimed-closest |
| **G6** | `object:type` + `combine:` of Claimed children | `cargo test -p arks --bin arks parse_filter` → `parse_filter_accepts_object_type_and_combine` **PASS** |

## Non-claims retained

tus / multi-tenant S3 LFS agents; `sparse:oid`; full forge UX; CDN bundle SLOs.
