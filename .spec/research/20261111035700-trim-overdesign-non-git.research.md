---
id: research:root/20261111035700
title: Trim over-design non-git product surfaces
status: active
created: 2026-11-10T19:00:00Z
updated: 2026-11-10T19:00:00Z
tags: [trim, overdesign, forge, host-env, lfs, celir, product-intent]
verify: host
rel:
  depends_on:
    - research:root/20261111034556
    - sot:root/product-intent
    - sot:root/cli-surface
  evidence: []
---

# Trim over-design non-git product surfaces

Tip base: **`68b733e`**. Owner directive: discard product surfaces that stock **`git` / `git-daemon`** do not support. Design intent was exceeded; triad remains (1) git-symmetric alias (2) race fixes (3) measured perf.

## Trimmed this change

| C-list | Action |
|--------|--------|
| C1/C2 `arks forge` / `forge recipe` | Removed HELP + dispatch; refuse with trim reason |
| C13 `arks host-env` | Removed module + HELP; refuse with trim reason |
| C14 `arks-hostfs` | Dropped from `default-members` and `arks` deps; crate remains non-default in-tree |
| C15 in-process `arks lfs` | Removed CLI module; refuse with trim reason; checkout smudge no longer auto-resolves LFS pointers (stock `filter.*.smudge` only) |
| C16 `serve-http` `info/lfs` | Removed server `lfs` module + HTTP route + unit tests; plain git HTTP **kept** |
| C17 CELIR product path | Honesty: ops/CI feature only (default OFF); not product Claimed |

## KEEP (unchanged intent)

- Full git CLI symmetry work
- `arks daemon` / `upload-pack` / `receive-pack` / `http-backend` / `serve-http` (git paths)
- `arks lease` / `arks lock` + measured perf
- Pilot fixes at `68b733e`

## Public / harness honesty

- `docs/git-vs-arks.md`, `completion.md`, `limits.md`, `contribute.md` — strip Claimed product language for trimmed rows
- Pilot P14 → SKIP; FS007/FS014 scripts → SKIP receipts

## Verification

Host: `cargo build -p arks -p arks-server -p arks-remote` + `cargo test -p arks-server` after trim.
