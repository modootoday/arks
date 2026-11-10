---
id: research:root/20261111033608
title: GitHub private remotes — git ↔ arks symmetry pilot
status: active
created: 2026-11-10T18:36:08Z
updated: 2026-11-10T19:00:00Z
tags: [pilot, github, private, symmetry, https, credentials]
verify: host
rel:
  depends_on:
    - sot:root/program-roadmap
    - sot:root/credential-helper-askpass
    - sot:root/cli-flag-parity-complete
    - docs:root/git-vs-arks
  evidence:
    - evidence:root/20261111033608
---

# GitHub private remotes — symmetry pilot

## Question

Against two **throwaway private** GitHub remotes, do stock **git** and **arks** show matching exits/refs/objects on Claimed surfaces (init/clone/fetch/push, branch porcelain, tags, shallow/partial, LFS, local dumb twin, bundle-uri)?

## Method

1. `gh repo create` under `modootoday` (private); HTTPS via `gh auth setup-git`.
2. Sequential matrix in `verify/scripts/run-gh-private-pilot-symmetry.ps1` — **Windows PowerShell only** (no `/bin/sh`); git then arks; record PASS/FAIL/SKIP+reason.
3. Linux-native scripted pilots stay in clean-room Docker; this host pilot uses `git.exe` / `arks.exe` / `gh`.
4. No G8 SLO / no G19 remasure / no stretch.
5. After wire fixes: **focused retest** of the four prior FAIL ids (AUTH-DIAG, P04abbrev, P05u, P08short) only — do not invent PASS for unretried rows.

## Result

See [[evidence:root/20261111033608]].

- Original at tip `fc12914`: **17 PASS / 4 FAIL / 5 SKIP**.
- After fixes + focused retest: **21 PASS / 0 FAIL / 5 SKIP** (four FAIL → PASS; other rows retained).

### Prior FAIL → fixed

| Id | Gap | Fix |
|----|-----|-----|
| AUTH-DIAG | Windows stock `!'…\gh.exe'` bang | Simple bang argv spawn (POSIX quotes stripped); cmd `/C` rewrite fallback |
| P04abbrev | `rev-parse --abbrev-ref` | Implement short ref name like git |
| P05u | `push -u` / `--set-upstream` | Set `branch.<name>.remote` + `.merge` after branch push |
| P08short | Short tag push | DWIM: `refs/heads/` then `refs/tags/` |

## Non-claims

GitHub is not a dumb-HTTP peer; exotic filters (`object:type`, `combine:`, `sparse:oid`) may be refused by the host for both clients — SKIP when both fail is not an arks-only asymmetry.
