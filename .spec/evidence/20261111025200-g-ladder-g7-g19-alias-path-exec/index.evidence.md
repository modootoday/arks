---
id: evidence:root/20261111025200
title: G7 sole Docker G19 — ALIAS PATH/exec absolute-shim cut
status: active
created: 2026-11-10T17:52:00Z
updated: 2026-11-10T17:52:00Z
tags: [g007-smart-http-client, g019-comparative-performance, pf5, alias, path-exec, g-ladder]
verify: docker
rel:
  depends_on:
    - plan:root/20261111024440
    - plan:root/20261111024441
    - sot:root/pf5-journey-seed-amortization
    - sot:root/program-roadmap
  evidence:
    - evidence:root/20261111015154
---

# G7 sole Docker G19 — ALIAS PATH/`exec` cut

## Product cut

Alias peer invokes hard-copied `git` shim by **absolute path** (documented preferred PATH-shim install) instead of PATH search — see `verify/gates/g016-journey-compare/tests/journey_compare.rs` `peer_command` + `docs/alias-git-arks.md`.

## Receipt

| Item | Value |
|------|-------|
| Venue | sole Docker `compose … run --rm g019-comparative-performance` |
| Tip at measure | `a02f7be` |
| Image | `arks-verify-m1:0.1.0` digest `sha256:5736c5687d3b5775e187ca3d21fbc5b2a5a2bf8965c5381778ee478eb185adf0` |
| Stock Git | 2.39.5 |
| Iterations | 3 |
| M17 ≤0.8× | **19/19 PASS** |
| Pf5 ≤0.5× claimed | **18** |
| `aggressive_na` | **J-ALIAS-01** only |
| J-ALIAS-01 | git=368 arks=202 ratio=**0.5489** → keep `aggressive_na` (was 0.5546) |
| J-REVERT-01 | ratio=**0.4943** Claimed |
| Thrash | **None** — exactly one sole G19; no second remasure |
| OOM | None (halt rule unused) |

Artifacts: `receipt.json`, `metrics.json` in this evidence directory.

## Honesty

PATH/`exec` cut improved ALIAS slightly but did **not** clear ≤0.5×. Tip remains **Claimed(partial)** with ALIAS `aggressive_na`. Do not stampede.
