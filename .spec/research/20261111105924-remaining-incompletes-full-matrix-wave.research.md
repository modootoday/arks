---
id: research:root/20261111105924
title: Remaining incompletes wave — hub full-matrix, personality CSV, daemon Refuse, docs lag
status: active
created: 2026-11-11T01:59:24Z
updated: 2026-11-11T01:59:24Z
tags: [research, residual, hub-full-matrix, personality, daemon, b5-plus, docs-honesty, no-invent-claimed, docker]
verify: manual
rel:
  depends_on:
    - research:root/20261111093543
    - research:root/20261111090827
    - sot:root/arks-verify-docker-e2e-claim-wave
    - sot:root/arks-partial-residual-wave
    - sot:root/arks-remaining-implement-wave
    - sot:root/arks-claimed-residual-gates
    - sot:root/production-residual-register
    - sot:root/verify-cleanroom-symmetry-matrix
    - sot:root/arks-hub-cli-surface
    - sot:root/arks-cli-personality-factory
    - sot:root/arks-daemon-product-intent
  related:
    - verify/matrices/hub-gh-arks-e2e-matrix.csv
    - verify/matrices/cli-git-help-matrix.csv
    - verify/matrices/daemon-git-arks-e2e-matrix.csv
---

# Remaining incompletes wave — web research

English-only. Tip surveyed: **`7b1f216`**. **Design ≠ Claimed.** Cite this note + [[sot:root/arks-remaining-incompletes-wave]] before crate Claimed flips.

Prior Docker e2e wave Claimed **IW-FORGE-SERVE**, **IW-HUB-FLAT subset**, **IW-PERSONALITY sample** ([[sot:root/arks-verify-docker-e2e-claim-wave]]). Leftovers at tip:

| ID | Tip honesty | Open bar |
|----|-------------|----------|
| **IW-HUB-FULL** | Claimed subset = 10 leaves; **v1 Missing = 80** | Inventory Missing→0 without inventing full-matrix Claimed |
| **IW-PERSONALITY-FULL** | Drop-in **sample** Claimed; cli-git-help CSV survey Missing=0 already | Broader personality peer sample + honest Partial for per-cmd tables |
| **IW-DAEMON-FLAGS / B5+** | Flags Partial; facility/priority + classic double-fork | Terminal **Refuse** unless collector evidence (none at tip) |
| **Docs lag** | README / completion still say forge-in-serve Partial, B5 Partial, SSH/forge rebind Partial | Sync to Claimed register + evidence ids |

## Sources (web / pin peers)

| Topic | URL | Fact used |
|-------|-----|-----------|
| `gh` command reference | https://cli.github.com/manual/gh_help_reference | Exhaustive leaf inventory; v1 groups = auth/pr/issue/release/repo/label/run/workflow/search/status/api/org/config/alias/completion |
| `gh api` passthrough | https://cli.github.com/manual/gh_api | Generic REST escape hatch — Partial API leaf ≠ per-leaf Claimed |
| REST roadmap pattern | https://docs.rs/crate/gor-cli/latest/source/docs/research/api-implementation-roadmap.md | Map leaves → REST; Actions/CI execute is deeper than list stubs |
| `gh` exit codes | https://cli.github.com/manual/gh_help_exit-codes | Drop-in classes 0/1/2/4; personality expand must preserve classes |
| Git porcelain vs plumbing | https://git-scm.com/book/en/v2/Git-Internals-Plumbing-and-Porcelain | Scripts need stable formats; personality `git` sample expand uses version/usage classes first |
| `git status --porcelain` | https://git-scm.com/docs/git-status | Machine-stable stdout class for optional expand sample |
| Git exit codes | https://stackoverflow.com/questions/4917871/does-git-return-specific-return-error-codes | Mostly 0/1; usage often 129/128-class — match pin exit **class**, not folklore |
| `git-daemon` docs | https://git-scm.com/docs/git-daemon | `--syslog` ≡ log-destination syslog; `--detach` implies syslog |
| `openlog` LOG_DAEMON | https://github.com/git/git/commit/0c591cacba00a36af797fa3b6c4a4f12d342be86 | Facility arming ≠ priority Claimed without collector |
| syslog(3) facility vs level | https://man7.org/linux/man-pages/man3/syslog.3.html | Facility ≠ level; Claimed needs collector receipt |
| git `daemonize` single-fork | https://public-inbox.org/git/E1G14LJ-0007q1-Fa@moooo.ath.cx/ | Peer detach = single fork + `setsid` — **not** classic SysV double-fork |
| glibc `daemon(3)` | https://man7.org/linux/man-pages/man3/daemon.3.html | Classic double-fork is **not** what git-daemon does |

Clean-room pin remains Git **2.39.5**. Hub muscle-memory pin remains host `gh` **2.103.3** (inventory only).

## Tip inventory (authoritative)

### Hub living CSV (`verify/matrices/hub-gh-arks-e2e-matrix.csv`)

| status_now (v1) | Count |
|-----------------|------:|
| Claimed | 10 (IW-HUB-FLAT subset) |
| Partial | 10 |
| Missing | **80** |
| (other phases Deferred/Refuse elsewhere) | — |

Many Missing rows already have **Partial client wiring** in `arks-hub` (label list/create, release list/create, run list/create, workflow list, org list, search repos/issues/prs) or **fail-closed honesty stubs** (`pr {other}`, `issue {other}` exit 2). CSV lag ≠ product Missing.

### CLI personality CSV (`verify/matrices/cli-git-help-matrix.csv`)

| survey_status | Count |
|---------------|------:|
| Missing | **0** |
| Partial | 117 |
| Refuse/N/A | 47 |
| Parity-candidate / synonym / arks-only | remainder |

Pillar-1 Missing=0 already. “Full personality CSV Missing” means **drop-in coverage beyond sample**, not a second Missing inventory.

### Daemon CSV

| status_now | Count |
|------------|------:|
| Missing | **0** |
| Partial | 15 |
| Refuse/N/A | 16 |
| Parity / arks-only | 8 |

IW-DAEMON-FLAGS deepen is inventory-closed; B5+ Claimed still absent (correct Refuse).

## Failure modes

| Mode | Prevention |
|------|------------|
| Invent full hub Claimed from Missing→Partial CSV edit | Claimed only with Docker receipt + subset/expanded list in SoT |
| Treat Partial stub exit-2 as Claimed leaf body | Claimed requires peer-faithful happy-path stdout/JSON |
| Expand personality Claimed from ExitCodePolicy units | Need `ARKS_PERSONALITY_DROPIN` / expand scenario receipt |
| Reopen B5+ without collector | Keep **Refuse**; document terminal |
| Docs lag inventing older Partial | Sync README/completion/self-host to residual Claimed ids |

## Options by residual class

### A — IW-HUB-FULL (v1 Missing→0)

| Option | Summary | Verdict |
|--------|---------|---------|
| **A1 Inventory honesty + Partial rephase + expanded Claimed subset** | Missing→Partial for wired/stub-honest leaves; Docker Claimed expand for REST-backed leaves beyond flat-10; Actions execute depth stay Partial/later | **Recommended** |
| **A2 Invent Claimed for all 80** | Mark Claimed without receipts | **Refuse** |
| **A3 Rephase all Missing→later** | Hides v1 must-have surface | **Refuse** (loses gh symmetry DoD) |

**Claimed bar (expand):** Docker `dod-hub-matrix` + `ARKS_HUB_IW_FULL=1` (or documented expand list) proves peer-faithful bodies for SoT-named leaves against local forge; receipt under `.spec/evidence/`. Full CSV may still contain Partial — Missing must be 0.

### B — IW-PERSONALITY-FULL

| Option | Summary | Verdict |
|--------|---------|---------|
| **B1 Expand Docker drop-in sample + Partial per-cmd tables** | Add git usage/status-class + gh issue/auth samples; keep per-cmd numeric tables Partial | **Recommended** |
| **B2 Claim every CLI Partial row as personality drop-in** | | **Refuse** (no peer oracle per row) |
| **B3 Treat cli Missing=0 as personality Done** | Sample-only still open in SoT | Incomplete — still expand sample |

### C — IW-DAEMON-FLAGS / B5+

| Option | Summary | Verdict |
|--------|---------|---------|
| **C1 Terminal Refuse close** | No collector; peer is single-fork; document Refuse | **Recommended** |
| **C2 Force facility Claimed** | | **Refuse** |

### D — Docs honesty

| Option | Summary | Verdict |
|--------|---------|---------|
| **D1 Sync README/docs to Claimed register** | IW-FORGE-SERVE / HUB-FLAT subset / PERSONALITY sample / B5 / B7 SSH+forge | **Required** this wave |

## Recommended wave order

1. Spec-first SoT [[sot:root/arks-remaining-incompletes-wave]]
2. Hub: CSV Missing→0 honesty + product deepen for expand Claimed leaves + Docker receipt
3. Personality: expand drop-in scenario + receipt
4. Daemon: SoT/docs terminal Refuse (no product Claimed)
5. Docs/register honesty sync

## Non-goals

- Product spawn of `git` / `gh` / `ssh` / `ollama`
- Origin push
- B5+ facility/priority Claimed without collector
- Classic SysV double-fork Claimed
- Inventing PASS from host PowerShell
