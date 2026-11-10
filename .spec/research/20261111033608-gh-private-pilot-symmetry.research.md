---
id: research:root/20261111033608
title: GitHub private remotes — git ↔ arks symmetry pilot
status: active
created: 2026-11-10T18:36:08Z
updated: 2026-11-10T18:50:00Z
tags: [pilot, github, private, symmetry, https, credentials]
verify: host
rel:
  depends_on:
    - sot:root/program-roadmap
    - docs:root/git-vs-arks
  evidence:
    - evidence:root/20261111033608
---

# GitHub private remotes — symmetry pilot

## Question

Against two **throwaway private** GitHub remotes, do stock **git** and **arks** show matching exits/refs/objects on Claimed surfaces (init/clone/fetch/push, branch porcelain, tags, shallow/partial, LFS, local dumb twin, bundle-uri)?

## Method

1. `gh repo create` under `modootoday` (private); HTTPS via `gh auth setup-git`.
2. Sequential matrix in `verify/scripts/run-gh-private-pilot-symmetry.ps1` — git then arks; record PASS/FAIL/SKIP+reason.
3. No G8 SLO / no G19 remasure / no stretch.

## Result

See [[evidence:root/20261111033608]]. **17 PASS / 4 FAIL / 5 SKIP** at tip `daf7f8a` (≥ `287d185`).

### Fix candidates (FAIL rows)

| Id | Gap |
|----|-----|
| AUTH-DIAG | Parse gitconfig bang-helpers with Windows-safe quoting (or strip shell quotes before `cmd /C`) |
| P04abbrev | Implement `rev-parse --abbrev-ref` |
| P05u | Implement `push -u` / `--set-upstream` |
| P08short | When pushing a name that is a tag, resolve `refs/tags/` (git shorthand) |

## Non-claims

GitHub is not a dumb-HTTP peer; exotic filters (`object:type`, `combine:`, `sparse:oid`) may be refused by the host for both clients — SKIP when both fail is not an arks-only asymmetry.
