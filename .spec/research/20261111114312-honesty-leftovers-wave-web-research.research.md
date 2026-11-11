---
id: research:root/20261111114312
title: Honesty leftovers wave — hub mutate REST, personality exit tables, B5+ collector reopen
status: active
created: 2026-11-11T02:43:12Z
updated: 2026-11-11T02:43:12Z
tags: [research, residual, hub-mutate, personality-exit-tables, daemon, b5-plus, syslog-collector, no-invent-claimed, docker]
verify: manual
rel:
  depends_on:
    - research:root/20261111105924
    - research:root/20261111093543
    - sot:root/arks-remaining-incompletes-wave
    - sot:root/arks-verify-docker-e2e-claim-wave
    - sot:root/arks-claimed-residual-gates
    - sot:root/production-residual-register
    - sot:root/arks-hub-cli-surface
    - sot:root/arks-cli-personality-factory
    - sot:root/arks-cli-output-exit-contract
    - sot:root/arks-daemon-product-intent
  related:
    - verify/matrices/hub-gh-arks-e2e-matrix.csv
    - verify/matrices/cli-git-help-matrix.csv
    - verify/matrices/daemon-git-arks-e2e-matrix.csv
---

# Honesty leftovers wave — web research

English-only. Tip surveyed: **`812d298`**. **Design ≠ Claimed.** Cite this note + [[sot:root/arks-honesty-leftovers-wave]] before crate Claimed flips.

Prior incompletes wave Claimed **IW-HUB-FULL** expand + **IW-PERSONALITY-FULL** expand; left honest Partial/Refuse:

| ID | Tip honesty (`812d298`) | Open bar |
|----|-------------------------|----------|
| **Hub non-expand v1** | Claimed=21; **Partial=79**; Missing=0 | Mutate/view depth beyond expand list (issue/pr close·reopen·edit·comment; release view; label edit/delete) — stub exit-2 ≠ Claimed |
| **Personality per-cmd exit tables** | Expand sample Claimed; ExitCodePolicy units ≠ tables | Named peer numeric rows + Docker receipt vs pin `git`/`gh` classes |
| **B5+ / IW-DAEMON-FLAGS** | Flags deepen Partial closed; **B5+ Refuse terminal** (openlog only; no collector) | Reopen **only** with collector Docker receipt proving facility+priority; else keep Refuse |
| **Other register Partials** | Clean-room tip Partial skew; daemon CSV Partial×15; Actions/asset/checkout; cli survey Partial×117 | Prefer honest Partial; no invent PASS |

## Sources (web / pin peers)

| Topic | URL | Fact used |
|-------|-----|-----------|
| GitHub REST pulls update/merge | https://docs.github.com/en/rest/pulls/pulls | `PATCH …/pulls/{n}` state/title/body; `PUT …/pulls/{n}/merge` metadata merge |
| GitHub REST issues update | https://docs.github.com/en/rest/issues/issues | `PATCH …/issues/{n}` state/title/body |
| GitHub REST issue comments | https://docs.github.com/en/rest/issues/comments | `POST …/issues/{n}/comments` body |
| GitHub REST release assets | https://docs.github.com/en/rest/releases/assets | Asset upload/download deeper than view — stay Partial this wave |
| `gh` exit-codes | https://cli.github.com/manual/gh_help_exit-codes | 0 success; 1 error; 2 cancel; 4 auth required; per-cmd may add (e.g. checks pending=8) |
| `gh` Main exit enum | https://github.com/cli/cli/blob/trunk/internal/ghcmd/cmd.go | Confirms 0/1/2/4/8 skeleton; Personality Claimed tables must cite classes + named rows |
| syslog(3) facility vs level | https://man7.org/linux/man-pages/man3/syslog.3.html | Facility (`LOG_DAEMON`) ≠ level (`LOG_INFO`); PRI = facility×8 + severity |
| rsyslog facility property | https://docs.rsyslog.com/doc/reference/properties/message-syslogfacility.html | Collector can assert numeric facility 3 = daemon |
| Docker syslog driver / facility | https://docs.docker.com/engine/logging/drivers/syslog/ | Facility arming alone ≠ Claimed without captured PRI |
| rsyslog collector container | https://docs.rsyslog.com/doc/containers/collector.html | Feasible UDP/TCP collector sidecar for clean-room evidence |
| `git-daemon` `--syslog` | https://git-scm.com/docs/git-daemon | Peer uses syslog destination; Claimed needs parity of facility+priority at collector |

Clean-room pin remains Git **2.39.5**. Hub inventory pin remains host `gh` **2.103.3** (inventory only).

## Tip inventory (authoritative working tree)

### Hub living CSV (`verify/matrices/hub-gh-arks-e2e-matrix.csv`)

| status_now (v1) | Count |
|-----------------|------:|
| Claimed | **21** (flat-10 + IW-HUB-FULL expand) |
| Partial | **79** |
| Missing | **0** |

Local forge (`arks-hub` serve) today: GET/POST for issues/pulls/labels/releases/runs; **no** PATCH issue/PR, **no** comments, **no** merge, **no** release-by-id GET in client leaves beyond list/create. Client `cmd_pr`/`cmd_issue` refuse non-list/view/create with exit 2.

### CLI personality CSV

| survey_status | Count |
|---------------|------:|
| Missing | **0** |
| Partial | **117** |
| Refuse/N/A | **47** |

Per-command peer numeric tables remain Partial after expand sample ([[evidence:root/20261111020515-personality-expand]]).

### Daemon CSV

| status_now | Count |
|------------|------:|
| Missing | **0** |
| Partial | **15** |
| Refuse/N/A | **16** |

Product `--syslog` arms `openlog(…, LOG_DAEMON)` only — **no** `syslog()` line + **no** collector receipt → B5+ correctly Refuse at tip.

## Failure modes

| Mode | Prevention |
|------|------------|
| Invent Claimed from stub exit-2 | Claimed requires peer-faithful happy-path body + Docker receipt |
| Claim Actions execute / assets / `pr checkout` | Explicit Stay Partial — no CI runner / no spawn git |
| Claim full 117 CLI Partial rows as exit tables | Named SoT row list only + Docker peer compare |
| Force B5+ without collector PRI | Keep Refuse; reopen only with collector evidence path |
| Treat unit ExitCodePolicy greens as table Claimed | Need `ARKS_PERSONALITY_EXIT_TABLES=1` Docker receipt |

## Options by residual class

### A — Hub non-expand mutate / view

| Option | Summary | Verdict |
|--------|---------|---------|
| **A1 Local-forge mutate subset Claimed** | Wire PATCH/POST comments (+ optional merge metadata) + release view + label edit/delete; Docker `ARKS_HUB_IW_MUTATE=1` | **Recommended** |
| **A2 Invent Claimed for all 79 Partial** | | **Refuse** |
| **A3 Leave all Partial forever** | Blocks honest REST depth already in peer | Incomplete vs wave goal |

**Stay Partial this wave:** `run`/`workflow` execute (watch/cancel/rerun/dispatch/enable/disable); release upload/download/delete-asset; `pr checkout`/`co` (must not spawn git); config/alias/completion installer depth; group-help-only rows; auth refresh.

### B — Personality per-command exit tables

| Option | Summary | Verdict |
|--------|---------|---------|
| **B1 Named peer exit-table subset Claimed** | Document rows in SoT; Docker compare exit **numbers** for named scenarios vs pin classes (`git` 0/1; `gh` 0/1/2/4) | **Recommended** |
| **B2 Claim every CLI Partial command** | | **Refuse** |
| **B3 Keep Partial only** | Allowed if evidence cannot land | Honest fallback |

### C — B5+ facility/priority

| Option | Summary | Verdict |
|--------|---------|---------|
| **C1 Collector reopen** | Emit `syslog(LOG_INFO, …)` after `openlog(LOG_DAEMON)`; Docker rsyslog/UDP collector asserts facility=3 + severity; receipt under `.spec/evidence/` | **Recommended if collector greens** |
| **C2 Keep Refuse terminal** | If collector path cannot land honestly in-wave | **Required fallback** — document; no invent Claimed |

### D — Other register Partials

| Option | Summary | Verdict |
|--------|---------|---------|
| **D1 Document-only honesty sync** | Residual register + product SoTs cite wave statuses | **Required** |
| **D2 Reopen live G8 / L9 / Impossible rows** | | **Refuse** this wave |

## Recommended wave order

1. Spec-first SoT [[sot:root/arks-honesty-leftovers-wave]] + this research
2. Hub mutate subset (store+serve+client+harness+Docker)
3. Personality named exit-table subset (SoT rows+harness+Docker)
4. B5+ collector attempt — Claimed only with PRI receipt; else Refuse document
5. Residual register / discovery / product SoT honesty sync

## Non-goals

- Product spawn of `git` / `gh` / `ssh` / `ollama`
- Origin push
- Inventing PASS from host PowerShell
- Classic SysV double-fork Claimed
- Actions CI execute Claimed
- Release asset binary Claimed
- Full CLI Partial→Claimed stampede
