---
id: evidence:root/20261109122512
title: Development residual close — version CODEOWNERS allocator dispositions
status: active
created: 2026-11-09T12:25:12Z
updated: 2026-11-10T00:57:43Z
tags: [residual, g21, version, codeowners, allocator, m16, m17, development-close, verified]
rel:
  depends_on: [sot:root/production-residual-register, sot:root/program-roadmap, sot:root/cli-flag-parity-complete, sot:root/public-surface, sot:root/crate-architecture, adr:root/20261109122126, adr:root/20261108232800, evidence:root/20261109090000, evidence:root/20261109061600, evidence:root/20261109063000]
  evidence: []
files:
  - {path: receipt-g21-sy5.json, sha256: 4e46a795037deffdd8864653dd6126e24333c648100e0e8507d1ed38f30ef2b8, bytes: 387, store: repo}
---

# Development residual close — version / CODEOWNERS / allocator / dispositions

Closes remaining **development** residuals after M0–M17 claimed exits. Normative register: [[sot:root/production-residual-register]] v0.6.0 (**no unfinished development Opens**).

## Implemented / verified

| Item | Disposition | Proof |
|------|-------------|-------|
| `arks version` / `--version` | **Closed (implement)** Sy-5 | Docker G21 PASS incl. **M16-SY5-05**; G20 + G14 regression |
| `.github/CODEOWNERS` | **Done** | Tracked file; [[sot:root/public-surface]] |
| Allocator helper identity | **Closed** ring-internal | [[adr:root/20261109122126]] |
| G8 / G9 | **Closed-deferred** (policy / unclaimed) | residual register criteria |
| Unclaimed concurrency catalog | **Closed-deferred out-of-milestone** | residual register + M13 plan |
| L3 N-way | **Refuse-closed / non-milestone** | [[adr:root/20261108232800]] |
| Server SSH / `git://` / dumb HTTP / v2 | **Refuse / N/A (Closed)** | [[sot:root/arks-server]] |
| M17 evidence cite | Points to [[evidence:root/20261109090000]] | program-roadmap + residual register |

## Environment

| Field | Value |
|-------|-------|
| Compose | `docker compose -f verify/docker-compose.yml run --rm g21` |
| Image | `arks-verify-m1:0.1.0` / `sha256:5736c5687d3b5775e187ca3d21fbc5b2a5a2bf8965c5381778ee478eb185adf0` |
| Stock Git | `git version 2.39.5` |
| rustc | `rustc 1.99.1 (58cf8f9c3 2026-10-13)` |
| Gate | G21 (Sy-1..Sy-5) + G20 + G14 regression — **PASS** |
| Specfold | `spec check` **0** errors / **0** warnings |
| Tip at receipt (pre-land) | `25ea53f3236495130a094baa682c68f2eaad4628` |
| Land tip | `0cfc21f16e744475b4e8dd0c2acbfa96851beacf` |

## Non-claims

- No L3 N-way ship; no G8 live forge without policy; no G9 filters without claim; no server SSH/`git://`/dumb HTTP/v2; no credential-helper/askpass/GUI/LFS/CELIR admin.
- Host greens advisory only; this receipt’s G21 row is Docker-authoritative.

## Reproduce

```bash
docker compose -f verify/docker-compose.yml run --rm g21
spec check
```
