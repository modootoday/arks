---
id: evidence:root/20261109070000
title: M17 Pf3-2 G19 progress (partial allow-list)
status: active
created: 2026-11-09T07:00:00Z
updated: 2026-11-10T00:57:42Z
tags: [g19, m17, pf3, progress, clean-room, perf]
rel:
  depends_on: [plan:root/20261109043916, evidence:root/20261109064500]
  evidence: []
files:
  - {path: receipt.json, sha256: 64a99601cac8f8037dc163b8587e7e6729a23b0530f0d2b177df57fbd8b7ba1e, bytes: 4350, store: repo}
---

# M17 Pf3-2 — G19 progress (not full exit)

## Engine / harness changes landed

- Match stock Git 2.39 default: no `fsync` on ref/index lock commits when `core.fsync` unset (Docker strace: `git tag` → 0 fsync; arks had 1× `sync_all` ≈15ms on bind mounts).
- Local path/`file://` clone: hardlink-or-copy (git-clone(1) local default); SoT amended.
- Checkout: skip rewriting blobs whose index OID already matches.
- Release: fat LTO, `panic=abort`, strip, sha1 asm.
- G16 harness: SoT-aligned untimed seeds; bulk amortization; alias shim via symlink.

## Latest Docker G19 snapshot (authoritative venue; tip dirty until land commit)

| Pass (≤0.8×) | J-CLONE-LOCAL-01, J-CLONE-PATH-01, J-REPACK-01, J-SUBMODULE-01 |
| N/A | J-TAG-01 (cold-start floor; G16 functional still required) |
| Fail | Remaining G16 IDs — process-startup dominated (arks `--help` ≈5ms vs `git version` ≈1ms) |

## Hotspot (blocking full M17)

Cold process-per-command floor: arks dyld/startup ≈4–6ms vs stock Git ≈1ms inside `arks-verify-m1:0.1.0`. Multi-step short porcelain cannot meet ≤0.8× without reducing process count or closing that floor.

## Reproduce

```bash
docker compose -f verify/docker-compose.yml run --rm g19
```
