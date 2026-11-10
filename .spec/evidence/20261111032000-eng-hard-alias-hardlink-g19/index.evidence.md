---
id: evidence:root/20261111032000
title: Eng-hard ALIAS hardlink/PATH cut — sole Docker G19 Claimed ≤0.5×
status: active
created: 2026-11-10T18:20:00Z
updated: 2026-11-10T18:20:00Z
tags: [g19, pf5, alias, hardlink, path-exec, eng-hard]
verify: docker
rel:
  depends_on:
    - adr:root/20261111031600
    - sot:root/pf5-journey-seed-amortization
    - sot:root/program-roadmap
    - evidence:root/20261111025200
---

# Eng-hard ALIAS hardlink/`exec` — sole Docker G19

## Product cut

1. Prefer **hard link** of release `arks` as `git` shim (copy fallback).
2. Invoke shim by **absolute path** with **no PATH env rewrite** per spawn.

Cite: `verify/gates/g16/tests/journey_compare.rs` `write_git_shim` / `peer_command`; [[adr:root/20261111031600]]; `docs/alias-git-arks.md`.

## Receipt

| Item | Value |
|------|-------|
| Venue | sole Docker `compose … run --rm g19` |
| Tip at measure | `9d764d7` (workspace dirty land tip) |
| Image | `arks-verify-m1:0.1.0` digest `sha256:5736c5687d3b5775e187ca3d21fbc5b2a5a2bf8965c5381778ee478eb185adf0` |
| Stock Git | 2.39.5 |
| Iterations | 3 |
| M17 ≤0.8× | **19/19 PASS** |
| Pf5 ≤0.5× claimed | **19** |
| `aggressive_na` | **0** (empty) |
| J-ALIAS-01 | git=406 arks=200 ratio=**0.4926** → **Claimed** (was 0.5489) |
| J-REVERT-01 | ratio=**0.4227** Claimed |
| Thrash | **None** — exactly one sole G19 |
| OOM | None |

Artifacts: `receipt.json`, `metrics.json` in this evidence directory.

## Honesty

Hardlink + no PATH rewrite cleared ALIAS ≤0.5×. Umbrella Pf5 tip **Claimed 19/19**. No second remasure.
