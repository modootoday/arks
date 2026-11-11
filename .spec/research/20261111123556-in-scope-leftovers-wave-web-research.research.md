---
id: research:root/20261111123556
title: In-scope leftovers wave — Phase 0 scope audit vs three symmetry pillars
status: active
created: 2026-11-11T03:35:56Z
updated: 2026-11-11T03:35:56Z
tags: [research, residual, scope-audit, hub, daemon, personality, no-invent-claimed, docker, design-scope]
verify: manual
rel:
  depends_on:
    - research:root/20261111114312
    - research:root/20261111105924
    - research:root/20261111093543
    - sot:root/arks-honesty-leftovers-wave
    - sot:root/arks-remaining-incompletes-wave
    - sot:root/arks-claimed-residual-gates
    - sot:root/verify-cleanroom-symmetry-matrix
    - sot:root/production-residual-register
    - sot:root/arks-hub-cli-surface
    - sot:root/arks-cli-personality-factory
    - sot:root/arks-daemon-product-intent
  related:
    - verify/matrices/hub-gh-arks-e2e-matrix.csv
    - verify/matrices/cli-git-help-matrix.csv
    - verify/matrices/daemon-git-arks-e2e-matrix.csv
    - sot:root/arks-in-scope-leftovers-wave
---

# In-scope leftovers wave — web research + tip inventory

English-only. Tip surveyed: **`484756a`**. **Design ≠ Claimed.** Cite this note + [[sot:root/arks-in-scope-leftovers-wave]] before crate Claimed flips.

**Design scope (normative):** the three DoD pillars in [[sot:root/verify-cleanroom-symmetry-matrix]] — **git↔arks**, **git-daemon↔arks daemon**, **gh↔hub** — plus **related serve/hosting** required for that symmetry. Out-of-scope rows are documented as non-goals; **do not invent work to Claim them**.

Prior honesty leftovers wave ([[research:root/20261111114312]] / [[sot:root/arks-honesty-leftovers-wave]]) Claimed **IW-HUB-MUTATE**, **IW-PERSONALITY-EXIT** named rows, **IW-B5-PLUS** collector. Explicit Stay Partial / Refuse left open for this wave to re-audit.

## Tip inventory (authoritative working tree @ `484756a`)

### Hub living CSV (`verify/matrices/hub-gh-arks-e2e-matrix.csv`)

| status_now / phase | Count |
|--------------------|------:|
| v1 Claimed | **33** |
| v1 Partial | **67** |
| v1 Missing | **0** |
| later Deferred | **119** |
| out-of-scope Refuse/N/A | **28** |
| later Partial | **0** |

v1 Partial leaves (authoritative list):

`auth`, `auth refresh`, `pr`, `pr status`, `pr diff`, `pr ready`, `pr review`, `pr checks`, `pr lock`, `pr unlock`, `pr update-branch`, `pr revert`, `pr checkout`, `co`, `issue`, `issue status`, `issue delete`, `issue lock`, `issue unlock`, `issue pin`, `issue unpin`, `issue transfer`, `release`, `release upload`, `release download`, `release edit`, `release delete`, `release delete-asset`, `repo`, `repo edit`, `repo delete`, `repo fork`, `repo rename`, `repo archive`, `repo unarchive`, `repo set-default`, `label`, `label clone`, `run`, `run list`, `run view`, `run watch`, `run cancel`, `run rerun`, `run download`, `run delete`, `workflow`, `workflow list`, `workflow view`, `workflow run`, `workflow enable`, `workflow disable`, `search`, `status`, `api`, `org`, `config`, `config get`, `config set`, `config list`, `config clear-cache`, `alias`, `alias set`, `alias list`, `alias delete`, `alias import`, `completion`.

### CLI survey (`verify/matrices/cli-git-help-matrix.csv`)

| survey_status | Count |
|---------------|------:|
| Partial | **117** |
| Refuse/N/A | **47** |
| Parity-candidate | **21** |
| arks-only | **6** |
| Synonym-wired | **4** |
| Missing | **0** |

Pillar DoD floor (Missing=0) already met ([[sot:root/verify-cleanroom-symmetry-matrix]]). Partial×117 = man-page / peer-depth residual — **in** git↔arks / personality DoD when Partial counts are tracked.

### Daemon CSV (`verify/matrices/daemon-git-arks-e2e-matrix.csv`)

| status_now | Count |
|------------|------:|
| Partial | **15** |
| Refuse/N/A | **16** |
| Parity | **5** |
| arks-only | **3** |
| Missing | **0** |

Partial peer-mapped flags: `--verbose`, `--timeout`, `--init-timeout`, `--max-connections`, `--strict-paths`, `--base-path`, `--reuseaddr`, `--enable`/`--disable`, `--listen`, `--port`, `<directory>…`, `service:upload-archive`, `service:receive-pack`, `related:git:// client`.

## Sources (web / pin peers)

| Topic | URL | Fact used |
|-------|-----|-----------|
| Actions workflow runs | https://docs.github.com/en/rest/actions/workflow-runs | `POST …/runs/{id}/cancel`, `…/rerun`; GET logs/artifacts redirect; list/view JSON |
| Actions workflows | https://docs.github.com/en/rest/actions/workflows | `PUT …/enable`, `…/disable`; `POST …/dispatches` with `ref` |
| Actions artifacts | https://docs.github.com/en/rest/actions/artifacts | artifact zip download redirect |
| Release assets | https://docs.github.com/en/rest/releases/assets | upload via `upload_url` raw body; download `Accept: application/octet-stream`; `DELETE …/assets/{id}` |
| `gh pr checkout` | https://cli.github.com/manual/gh_pr_checkout | Peer fetches `refs/pull/<n>/head` via git; arks **MUST NOT** spawn git — Claimed = resolve/print head SHA/ref (+ optional in-process arks checkout) |
| git-daemon flags | https://git-scm.com/docs/git-daemon | `--verbose`, timeouts, `--listen`, `--max-connections`, `--strict-paths`, enable/disable services |
| git-daemon detach | https://git-scm.com/docs/git-daemon + prior [[research:root/20261111093543]] | Peer `--detach` = single-fork + `setsid` — **not** classic SysV double-fork |
| Symmetry pillars | [[sot:root/verify-cleanroom-symmetry-matrix]] | CLI / daemon / hub + related self-host; smoke ≠ Claimed |
| Honesty leftovers Stay Partial | [[sot:root/arks-honesty-leftovers-wave]] | Actions/assets/checkout were Stay Partial **that wave** — reopened here as in-scope when matrix rows exist |

Clean-room pin remains Git **2.39.5**. Hub inventory pin remains host `gh` **2.103.3** (inventory only).

## Scope decision rules (Phase 0)

| Rule | In-scope? |
|------|-----------|
| Hub v1 Partial with living CSV row under gh↔hub pillar | **YES** (gh) |
| Hub later Deferred×119 | **OUT** (phase=later; not v1 DoD) |
| Hub out-of-scope Refuse (`extension`/`copilot`/…) | **OUT** |
| CLI survey Partial×117 (Missing=0 already) | **YES** (git / personality) when Partial counts are DoD residual |
| Daemon Partial flags that map to git-daemon peers | **YES** (daemon) |
| Daemon Refuse that documents non-goals (inetd, privilege drop, templates, …) | **OUT** of Claimed bar (keep Refuse) |
| Classic SysV double-fork Claimed | **OUT** / terminal Refuse (peer = single-fork+`setsid`) |
| Live G8 secrets, L9, Impossible crypto/host rows | **OUT** |
| Related serve/hosting already Claimed (IW-FORGE-SERVE / B7 / B9) | No reopen unless residual Partial with pillar cite |

### Claimed honesty (prevent invent)

| Class | Claimed means | Refuse as Claimed |
|-------|---------------|-------------------|
| Actions execute | Local-forge **metadata** lifecycle (view/cancel/rerun/delete/watch-poll status; workflow enable/disable/dispatch creating a run record) | Inventing a real CI runner / Actions compute |
| Release assets | Local-forge binary store upload/download/delete-asset + client leaves | Silent stub exit-2 marketed Claimed |
| `pr checkout` / `co` | Resolve PR head (SHA/ref/branch) + print recipe and/or **in-process** `arks` checkout | Spawn `git`/`gh`; invent worktree Claimed without evidence |
| Config/alias/completion | Peer-faithful get/set/list/delete or documented completion emit | Installer/profile mutation invent |
| CLI Partial×117 | Named expand rows with Docker peer compare; Missing=0 already | Stampede Claimed for all 117 without receipts |
| Daemon Partial→Parity | Docker peer behavior for named flags | Classic double-fork; invent Parity from clap accept alone |

## Failure modes

| Mode | Prevention |
|------|------------|
| Invent Claimed from stub exit-2 | Require peer-faithful body + Docker receipt |
| Treat later Deferred as v1 must-Claim | Scope table marks OUT |
| Reopen L9 / live secrets / Impossible | Explicit OUT; no product work |
| Claim SysV double-fork | Terminal Refuse — peer setsid |
| Actions compute Claimed | Metadata-only Claimed bar |
| Docker thrash | Stop after one classified mismatch |

## Recommended wave order (Phase 1+)

1. Spec-first [[sot:root/arks-in-scope-leftovers-wave]] + this research (this commit)
2. **IW-HUB-ACTIONS** local-forge metadata execute + Docker
3. **IW-HUB-ASSETS** release assets + Docker
4. **IW-HUB-CHECKOUT** resolve/print (+ optional in-process) + Docker
5. **IW-HUB-UX** config/alias/completion Claimed subset + Docker
6. **IW-HUB-STUBS** remaining v1 Partial REST deepen where peer-faithful; else honest Partial cite
7. **IW-PERSONALITY-SURVEY** named Partial→Claimed expand beyond exit tables; remainder honest Partial
8. **IW-DAEMON-FLAGS-PARITY** Partial→Parity for peer-mapped flags with Docker; Refuse rows untouched
9. Residual register / discovery / product SoT honesty sync

## Non-goals

- Product spawn of `git` / `gh` / `ssh` / `ollama`
- Origin push
- Inventing PASS from host PowerShell
- Classic SysV double-fork Claimed
- Live G8 / L9 / Impossible crypto Claimed
- Graduating OUT hub groups (`extension` / `copilot` / …)
- Claiming later-phase Deferred×119 as v1
