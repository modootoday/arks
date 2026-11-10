---
id: evidence:root/20261111033608
title: GitHub private-repo git↔arks symmetry pilot (Windows host)
status: active
created: 2026-11-10T18:36:08Z
updated: 2026-11-10T19:00:00Z
tags: [pilot, symmetry, github, private, windows, https]
verify: host-windows
rel:
  depends_on:
    - sot:root/program-roadmap
    - sot:root/credential-helper-askpass
    - sot:root/cli-flag-parity-complete
    - evidence:root/20261111032000
---

# GitHub private pilot — git ↔ arks symmetry

Bounded functional symmetry against **two throwaway private** remotes. **Not** G8 latency SLO / **not** G19 remasure / **not** stretch.

## Host constraint

| Item | Value |
|------|-------|
| Host OS | Windows 10/11 |
| Runner | PowerShell only (`verify/scripts/run-gh-private-pilot-symmetry.ps1`) — **no `/bin/sh`** |
| Clients | stock `git.exe` + `target/debug/arks.exe` (focused fix retest) / prior `target/pilot-bin/arks.exe` |
| Protocol | **HTTPS** via `gh auth` (`gh auth setup-git`); SSH skipped (agent key not accepted by github.com) |
| Credential | stock `!'…\gh.exe'` bang + optional `ARKS_CREDENTIAL_HELPER=!gh auth git-credential` |

## Remotes (left in place)

| Role | URL | Visibility |
|------|-----|------------|
| A (non-LFS matrix) | https://github.com/modootoday/arks-pilot-a-20261111033608 | private |
| B (LFS) | https://github.com/modootoday/arks-pilot-b-20261111033608 | private |

**Clone disposition:** repos **kept**; descriptions mark throwaway/pilot. Do not delete unless operator asks.

## Score

| PASS | FAIL | SKIP |
|------|------|------|
| **21** | **0** | **5** |

Original matrix at tip `fc12914` / prior `daf7f8a`: **17 PASS / 4 FAIL / 5 SKIP**. Focused fix retest flipped the four FAIL rows to PASS (AUTH-DIAG, P04abbrev, P05u, P08short); other PASS/SKIP rows retained from the original run (not invented).

## Matrix

| Id | Scenario | Status | Notes |
|----|----------|--------|-------|
| S00 | seed push (git HTTPS) | **PASS** | force allowed on throwaway only |
| AUTH-DIAG | stock `!'…\gh.exe'` vs `!gh` PATH fill | **PASS** | fix retest: argv spawn honors POSIX-quoted bang |
| P01 | init | **PASS** | |
| P02 | clone smart HTTPS | **PASS** | git≡arks HEAD |
| P03 | fetch | **PASS** | |
| P04 | branch/checkout/commit/log | **PASS** | branch via `symbolic-ref` |
| P04abbrev | `rev-parse --abbrev-ref` | **PASS** | fix retest: short branch name |
| P05u | `push -u` | **PASS** | fix retest: upstream config written |
| P05 | push topics (HTTPS) | **PASS** | original run; see P05u for `-u` |
| P06 | merge | **PASS** | |
| P07 | rebase (non-interactive) | **PASS** | |
| P08short | `push origin <tagname>` | **PASS** | fix retest: tag DWIM |
| P08 | annotated tag + `refs/tags/` push | **PASS** | |
| P08s | signed tag (`gpg.format=ssh`) | **PASS** | |
| P09 | `--depth 1` | **PASS** | |
| P10a | `--filter=blob:none` | **PASS** | |
| P10b | `--filter=tree:0` | **PASS** | |
| P10c | `--filter=object:type=blob` | **SKIP** | both fail on GitHub |
| P10d | `--filter=combine:…` | **SKIP** | both fail on GitHub |
| P10e | `--filter=sparse:oid=` | **SKIP** | both fail on GitHub |
| P11 | LFS clone/smudge | **PASS** | Repo B; size match 2048 |
| P11L | LFS locks | **SKIP** | GitHub lock unauthorized/failed this run |
| P12 | dumb HTTP (local `serve-http --dumb-only`) | **PASS** | not faked on GitHub |
| P13 | `--bundle-uri` (local file) | **PASS** | |
| P14 | `host-env` advisory | **PASS** | N/A as GitHub hazard; local probe |
| P15 | SSH clone | **SKIP** | SSH to github.com unavailable here |

## Fixes landed (was asymmetries)

1. **Credential helper bang quotes (Windows)** — simple bang argv (incl. POSIX single-quoted `gh.exe` path) spawns directly; no host `sh` required.
2. **`rev-parse --abbrev-ref`** — short branch/tag/remote name like git.
3. **`push -u` / `--set-upstream`** — writes `branch.<name>.remote` + `.merge` after branch push.
4. **Tag push shorthand** — short name DWIM: heads first, else `refs/tags/`.

## Not tested (out of scope)

- G8 live forge latency SLO / remasure thrash  
- G19 performance remasure / Pf5 stretch  
- L9 / stress  
- Forge host UX (Checks / merge-queue / Issues)  
- Dumb HTTP **on GitHub** (smart-only; dumb = local twin)  
- Interactive rebase / `add -p`  
- Linux-native `/bin/sh` pilots on this host (use Docker clean-room when needed)

## Artifacts

- `receipt.json`, `results.csv`, `summary.md`, `logs/`  
- Runner: `verify/scripts/run-gh-private-pilot-symmetry.ps1` (Windows PowerShell; no sh)
