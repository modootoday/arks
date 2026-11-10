---
id: evidence:root/20261111033608
title: GitHub private-repo git↔arks symmetry pilot (Windows host)
status: active
created: 2026-11-10T18:36:08Z
updated: 2026-11-10T18:50:00Z
tags: [pilot, symmetry, github, private, windows, https]
verify: host-windows
rel:
  depends_on:
    - sot:root/program-roadmap
    - evidence:root/20261111032000
---

# GitHub private pilot — git ↔ arks symmetry

Bounded functional symmetry against **two throwaway private** remotes. **Not** G8 latency SLO / **not** G19 remasure / **not** stretch.

## Host constraint

| Item | Value |
|------|-------|
| Host OS | Windows 10/11 |
| Runner | PowerShell only (`verify/scripts/run-gh-private-pilot-symmetry.ps1`) — **no `/bin/sh`** |
| Clients | stock `git.exe` + `target/pilot-bin/arks.exe` (debug rebuild at tip; release LTO link ACCESS_VIOLATION on this host) |
| Protocol | **HTTPS** via `gh auth` (`gh auth setup-git`); SSH skipped (agent key not accepted by github.com) |
| Credential override | `ARKS_CREDENTIAL_HELPER=!gh auth git-credential` (PATH bang; see AUTH-DIAG) |

## Remotes (left in place)

| Role | URL | Visibility |
|------|-----|------------|
| A (non-LFS matrix) | https://github.com/modootoday/arks-pilot-a-20261111033608 | private |
| B (LFS) | https://github.com/modootoday/arks-pilot-b-20261111033608 | private |

**Clone disposition:** repos **kept**; descriptions mark throwaway/pilot. Do not delete unless operator asks.

## Score

| PASS | FAIL | SKIP |
|------|------|------|
| **17** | **4** | **5** |

Tip recorded in `receipt.json`: `daf7f8a63e8c0422617be3a93064c075db816a1f` (≥ `287d185`).

## Matrix

| Id | Scenario | Status | Notes |
|----|----------|--------|-------|
| S00 | seed push (git HTTPS) | **PASS** | force allowed on throwaway only |
| AUTH-DIAG | stock `!'…\gh.exe'` vs `!gh` PATH fill | **FAIL** | arks `cmd /C` ignores shell single-quotes → empty fill → HTTP 401 without override |
| P01 | init | **PASS** | |
| P02 | clone smart HTTPS | **PASS** | git≡arks HEAD |
| P03 | fetch | **PASS** | |
| P04 | branch/checkout/commit/log | **PASS** | branch via `symbolic-ref` |
| P04abbrev | `rev-parse --abbrev-ref` | **FAIL** | arks unsupported option |
| P05u | `push -u` | **FAIL** | arks unsupported `-u` |
| P05 | push topics (HTTPS) | **PASS** | arks plain `push` (no `-u`) |
| P06 | merge | **PASS** | |
| P07 | rebase (non-interactive) | **PASS** | |
| P08short | `push origin <tagname>` | **FAIL** | arks looks up `refs/heads/<name>` |
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

## Asymmetries (bugs / gaps to fix)

1. **Credential helper bang quotes (Windows)** — stock `credential.https://github.com.helper=!'C:\Program Files\GitHub CLI\gh.exe' auth git-credential` does not fill under arks (`cmd /C`). Workaround: `ARKS_CREDENTIAL_HELPER=!gh auth git-credential` with `gh` on PATH.
2. **`rev-parse --abbrev-ref`** — unsupported; `symbolic-ref HEAD` works.
3. **`push -u` / `--set-upstream`** — unsupported.
4. **Tag push shorthand** — `arks push origin <tag>` resolves `refs/heads/<tag>`; need `refs/tags/<tag>`.

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
