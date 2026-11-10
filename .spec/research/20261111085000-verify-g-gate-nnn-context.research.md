---
id: research:root/20261111085000
title: Verify G-gate ids — three-digit pad + contextual kebab survey
status: active
created: 2026-11-11T08:50:00Z
updated: 2026-11-11T08:50:00Z
tags: [verify, clean-room, conventions, naming, numeric, zero-pad, gates, survey, migration]
verify: manual
rel:
  depends_on:
    - sot:root/verify-directory-conventions
    - research:root/20261111082440
    - sot:root/product-verification-clean-room
    - sot:root/language-english-only
  evidence: []
---

# Verify G-gate ids — three-digit pad + contextual kebab survey

Dated survey feeding [[sot:root/verify-directory-conventions]] **0.5.0**. **Does not invent Claimed/PASS.** **Does not mass-rename** the live `verify/gates/` tree.

## Problem

1. Opaque bare ids (`g011-lease-product`, `g019-comparative-performance`, `g041-apache-dav-interop`) do not convey concern when browsing `verify/gates/`.
2. Unpadded `g001-loose-object-interop`…`g010-store-facade`…`g002-refs-index-roundtrip` sort wrongly in GitHub UI / lexicographic listings (`g001-loose-object-interop`, `g010-store-facade`, `g011-lease-product`, `g002-refs-index-roundtrip`, …).
3. Revision 0.4.0 forbade G-series zero-pad; interim two-digit `gNN` guidance also insufficient vs `fsNNN`.

## Normative direction (SoT)

- G-series **MUST** use three-digit zero-padded `gNNN` (`g001`…`g999`) — **same width class as `fsNNN`**.
- **New** gates **MUST** be `gNNN-<kebab-context>` (short English concern slug).
- Live unpadded / bare ids are **grandfathered** until a Migration wave.
- Compose / `run-*.sh` / matrix stems **MUST** share the same contextual id when present.

## Example rename proposals (non-MUST)

| Live | Proposed |
|------|----------|
| `g001-loose-object-interop` | `g001-loose-object-interop` |
| `g011-lease-product` | `g011-lease-product` |
| `g015-smart-http` | `g015-smart-http` |
| `g019-comparative-performance` | `g019-comparative-performance` |
| `g032-git-daemon-wire` | `g032-git-daemon-wire` |
| `g033-cred-askpass` | `g033-cred-askpass` |
| `g041-apache-dav-interop` | `g041-apache-dav-interop` |
| `g008-live-scale` | `g008-live-scale` |
| `g028-offline-clones` | `g028-offline-clones` |
| `g011-high-concurrency` | `g011-l3` |

Full table: SoT Appendix D. Coordinate with verify-migration siblings; Spec-only waves amend SoT only.

## Sources

- `verify/README.md` gate table
- `verify/scripts/run-g*.sh` headers
- `verify/matrices/matrix-g*.md` titles
- Tip ≥ `9b27ea5` committed tree survey
