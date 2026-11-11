---
id: sot:root/arks-in-scope-leftovers-wave
title: In-scope leftovers wave — Phase 0 scope audit + Phase 1+ Claimed bars
status: active
created: 2026-11-11T03:35:56Z
updated: 2026-11-11T03:35:56Z
tags: [design, residual, scope-audit, hub, daemon, personality, no-invent-claimed, docker, spec-first]
verify: manual
rel:
  depends_on:
    - research:root/20261111123556
    - research:root/20261111114312
    - sot:root/arks-honesty-leftovers-wave
    - sot:root/arks-remaining-incompletes-wave
    - sot:root/arks-claimed-residual-gates
    - sot:root/production-residual-register
    - sot:root/verify-directory-conventions
    - sot:root/verify-cleanroom-symmetry-matrix
    - sot:root/arks-hub-cli-surface
    - sot:root/arks-hub-gh-command-matrix
    - sot:root/arks-cli-personality-factory
    - sot:root/arks-cli-output-exit-contract
    - sot:root/arks-daemon-product-intent
    - sot:root/arks-self-host-server
    - sot:root/product-verification-clean-room
    - sot:root/language-english-only
    - sot:root/public-surface
  governs:
    - verify/scripts/run-hub-gh-arks-matrix-e2e.sh
    - verify/scripts/run-cli-git-arks-matrix-e2e.sh
    - verify/scripts/run-daemon-git-arks-matrix-e2e.sh
    - verify/matrices/hub-gh-arks-e2e-matrix.csv
    - verify/matrices/cli-git-help-matrix.csv
    - verify/matrices/daemon-git-arks-e2e-matrix.csv
---

# In-scope leftovers wave — design SoT

Document revision **0.1.0** (SoT edit counter only — **not** a product/crate release). English-only.

**This SoT gates leftovers after [[sot:root/arks-honesty-leftovers-wave]] tip `484756a`.** Phase 0 = scope audit (this document). Phase 1+ Claimed flips cite evidence + [[research:root/20261111123556]].

**Design scope:** git / git-daemon / gh symmetry pillars + related serve/hosting required for that symmetry ([[sot:root/verify-cleanroom-symmetry-matrix]]). Out-of-scope → document non-goal; **do not invent work to Claim them**.

## Rule

### Absolute honesty

1. Stub exit-2, CSV edits alone, ExitCodePolicy unit greens, clap-accept-only, and host PowerShell **MUST NOT** invent Claimed / Parity.
2. Product paths **MUST NOT** spawn `git` / `gh` / `ssh` / `ollama`. Verify **MAY** compare pin peers.
3. Clean-room Linux Docker (`arks-verify-m1` + Git pin **2.39.5**) is normative DoD for Claimed / heavy verify.
4. New/extended slots follow [[sot:root/verify-directory-conventions]] — Spec first, then harness.
5. Prefer honest **Partial** / **Refuse** over invent Claimed. **No origin push.**
6. Close **in-scope** leftovers to Claimed **where evidence allows**; honest Partial only with SoT cite.

### Phase 0 — Scope audit (normative)

Tip inventory @ `484756a` ([[research:root/20261111123556]]):

| Ledger | Open residual |
|--------|---------------|
| Hub v1 | Partial **67**; Missing **0**; Claimed **33** |
| Hub later | Deferred **119** |
| Hub OUT groups | Refuse/N/A **28** |
| CLI survey | Partial **117**; Missing **0** |
| Daemon | Partial **15**; Refuse/N/A **16**; Parity **5** |

#### Scope table — EVERY remaining Partial / Missing / Refuse

| ID / surface | Pillar | In-scope? | Why (SoT cite) |
|--------------|--------|-----------|----------------|
| **H-ACT** `run view\|watch\|cancel\|rerun\|download\|delete` (+ list Partial deepen) | gh | **YES** | Living v1 CSV rows; gh↔hub pillar ([[sot:root/verify-cleanroom-symmetry-matrix]] §hub; [[sot:root/arks-hub-cli-surface]]). Claimed = local-forge **metadata** Actions — not CI compute |
| **H-WF** `workflow view\|run\|enable\|disable` (+ list) | gh | **YES** | Same pillar; dispatch/enable/disable metadata on local forge |
| **H-ASSET** `release upload\|download\|delete-asset` (+ `release edit\|delete`) | gh | **YES** | Matrix rows; peer release assets REST ([[research:root/20261111123556]]) |
| **H-CO** `pr checkout`, `co` | gh | **YES** | Matrix rows; Claimed = resolve/print (+ optional in-process arks checkout); **never** spawn git ([[sot:root/arks-honesty-leftovers-wave]] Stay Partial reopen) |
| **H-UX** `config *`, `alias *`, `completion` | gh | **YES** | Matrix rows; forge/CLI UX parity ([[sot:root/arks-hub-cli-surface]]) |
| **H-STUB-PR** `pr status\|diff\|ready\|review\|checks\|lock\|unlock\|update-branch\|revert` | gh | **YES** | v1 Partial stubs; deepen peer-faithful REST/metadata where local forge can; else honest Partial cite |
| **H-STUB-ISSUE** `issue status\|delete\|lock\|unlock\|pin\|unpin\|transfer` | gh | **YES** | Same |
| **H-STUB-REPO** `repo edit\|delete\|fork\|rename\|archive\|unarchive\|set-default` | gh | **YES** | Same; never spawn git |
| **H-STUB-MISC** `label clone`, `auth refresh`, `status`, `api`, `search`/`org`/`label`/`release`/`run`/`workflow`/`pr`/`issue`/`repo`/`auth` group-help | gh | **YES** | Inventory in pillar; group-help Claimed bar = discoverable help exit 0; `auth refresh` may stay Partial (OAuth) with cite |
| **H-LATER** Deferred×119 (`phase=later`) | OUT | **NO** | Not v1 DoD; phase demotion ([[sot:root/arks-hub-gh-command-matrix]]); do not invent Claimed |
| **H-OUT** `extension*` / `copilot` / `skill*` / `agent-task*` / `auth setup-git` / `licenses` / `preview*` | OUT | **NO** | CSV `out-of-scope` Refuse; [[sot:root/arks-claimed-residual-gates]] non-goals |
| **C-SURVEY** CLI survey Partial×117 | git | **YES** | Pillar tracks Partial after Missing=0 ([[sot:root/verify-cleanroom-symmetry-matrix]]; [[sot:root/arks-cli-personality-factory]]). Named expand Claimed; remainder honest Partial |
| **C-REFUSE** CLI Refuse/N/A×47 | OUT | **NO** | Explicit refuse-with-reason inventory |
| **D-FLAGS** Daemon Partial×15 peer-mapped (`--verbose`, timeouts, listen/port, max-connections, strict-paths, base-path, reuseaddr, enable/disable, dirs, upload-archive/receive-pack services, git:// client pointer) | daemon | **YES** | Daemon pillar deepen Partial→Parity ([[sot:root/arks-daemon-product-intent]]; [[sot:root/arks-daemon-git-command-matrix]]) |
| **D-REFUSE** Daemon Refuse (inetd, privilege drop, templates, user-path, base-path-relaxed, access-hook, informative-errors toggle, pid-file, daemon.* override, …) | OUT | **NO** | Documented non-goals; keep Refuse |
| **D-DETACH / SysV double-fork** | OUT | **NO** | Terminal Refuse — peer `--detach` = single-fork+`setsid` ([[sot:root/arks-daemon-product-intent]] rule 10; [[research:root/20261111093543]]) |
| **D-SYSLOG dest CSV** historical Refuse row vs B5+ Claimed | daemon / docs | **honesty sync** | Facility Claimed elsewhere ([[evidence:root/20261111025206-daemon-b5-plus-syslog]]); CSV note may lag — sync in Phase 1+ docs, do not reopen invent |
| **R-G8 / L9 / Impossible crypto·host** | OUT | **NO** | [[sot:root/production-residual-register]] next-queue; secrets-blocked / Impossible |
| **R-SERVE** self-host / B7 / B9 / IW-FORGE-SERVE | serve-related | **closed** | Already Claimed; no invent reopen unless new Partial with cite |

**Counts (Phase 0):**

| Bucket | Count (approx) |
|--------|----------------|
| In-scope open (hub v1 Partial + CLI Partial + daemon Partial) | **67 + 117 + 15 = 199** surface rows |
| Out-of-scope / terminal Refuse / later / closed | Hub later 119 + hub OUT 28 + CLI Refuse 47 + daemon Refuse 16 + SysV/L9/G8/Impossible + serve already Claimed |

### Ordered backlog (Phase 1+)

| Order | ID | Surface | Status now | Close bar | Refuse / non-goal | Docker / verify |
|------:|----|---------|------------|-----------|-------------------|-----------------|
| 1 | **IW-HUB-ACTIONS** | Actions execute metadata | Partial | Local-forge view/cancel/rerun/delete + watch-poll status; workflow view/enable/disable/dispatch→run record; Docker receipt | Invent CI runner / Actions compute | `dod-hub-matrix` + `ARKS_HUB_IW_ACTIONS=1` |
| 2 | **IW-HUB-ASSETS** | Release assets (+ edit/delete) | Partial | Upload/download/delete-asset peer-faithful vs local forge; edit/delete release; Docker receipt | Stub exit-2 Claimed | `dod-hub-matrix` + `ARKS_HUB_IW_ASSETS=1` |
| 3 | **IW-HUB-CHECKOUT** | `pr checkout` / `co` | Partial | Resolve head SHA/ref/branch; print recipe and/or in-process arks checkout; Docker receipt | Spawn git/gh | `dod-hub-matrix` + `ARKS_HUB_IW_CHECKOUT=1` |
| 4 | **IW-HUB-UX** | config / alias / completion | Partial | get/set/list/delete (+ completion emit) Claimed subset; Docker receipt | Profile installer invent | `dod-hub-matrix` + `ARKS_HUB_IW_UX=1` |
| 5 | **IW-HUB-STUBS** | Remaining v1 Partial stubs | Partial | Peer-faithful REST/metadata for SoT expand list; else honest Partial cite | Invent full-matrix Claimed | `dod-hub-matrix` + `ARKS_HUB_IW_STUBS=1` |
| 6 | **IW-PERSONALITY-SURVEY** | CLI Partial×117 | Partial | Named expand Claimed rows beyond IW-PERSONALITY-EXIT; Missing=0 retained; remainder Partial with cite | Stampede 117 Claimed | `dod-cli-matrix` + `ARKS_PERSONALITY_SURVEY=1` |
| 7 | **IW-DAEMON-FLAGS-PARITY** | Daemon Partial×15 | Partial | Docker peer behavior → Parity for named flags; services deepen | SysV double-fork; clap-accept-only Parity | `dod-daemon-matrix` + `ARKS_DAEMON_FLAGS_PARITY=1` |
| 8 | **DOCS-SCOPE** | Register + product SoTs | — | Sync in/out counts + evidence ids | Invent beyond receipts | Manual |

### IW-HUB-ACTIONS detail

#### Normative Claimed expand list

| Leaf | Peer-faithful bar (local forge) |
|------|----------------------------------|
| `run view` | GET run by id; JSON includes `id`/`status` |
| `run cancel` | POST cancel → status canceled/completed honesty |
| `run rerun` | POST rerun → new or reset run record |
| `run delete` | DELETE run; subsequent list omits |
| `run watch` | Poll until terminal status (bounded); print status lines |
| `run download` | Download stored artifact/log bytes if forge has them; else honest Partial cite |
| `workflow view` | GET workflow by id/path |
| `workflow enable` / `disable` | PUT state active/disabled |
| `workflow run` | Dispatch creates run record (`ref` required) |

**Refuse as Claimed:** real GitHub Actions compute; invent runner; CSV-only greens.

### IW-HUB-ASSETS detail

| Leaf | Bar |
|------|-----|
| `release upload` | POST raw asset to local forge `upload_url`; response includes `name`/`size` |
| `release download` | GET octet-stream / file bytes |
| `release delete-asset` | DELETE asset id; list omits |
| `release edit` / `delete` | PATCH/DELETE release metadata |

### IW-HUB-CHECKOUT detail

| Leaf | Bar |
|------|-----|
| `pr checkout` / `co` | Resolve PR → head SHA + ref/branch; stdout recipe **or** in-process `arks` checkout of that head; exit 0 on success |
| Spawn | **Forbidden** (`git`/`gh`) |

### IW-HUB-UX detail

| Leaf | Bar |
|------|-----|
| `config get\|set\|list\|clear-cache` | Local forge-client config file peer-faithful |
| `alias set\|list\|delete\|import` | Alias store round-trip |
| `completion` | Emit shell completion script for `arks`/`hub` leaves (stdout); not installer mutation |

### IW-HUB-STUBS expand candidates (Claimed only with receipt)

`pr diff`, `pr checks`, `pr ready`, `pr review` (metadata), `pr status`, `issue status`, `issue delete`, `repo edit` (PATCH metadata), `repo set-default`, `api` GET passthrough deepen, `status` aggregated inbox — **only** when local forge supports peer-faithful bodies.

**Stay Partial with cite when no honest path:** `pr lock|unlock|update-branch|revert`, `issue lock|pin|transfer|…`, `repo fork|delete|rename|archive|…` requiring git spawn or non-local semantics, `auth refresh` OAuth, `label clone`.

Group-help-only rows: Claimed help discoverability **or** remain Partial — do not invent leaf depth.

### IW-PERSONALITY-SURVEY detail

| Scenario | Bar |
|----------|-----|
| Retain Missing=0 | Already met — do not reopen Missing |
| Named expand | Additional peer exit/stdout rows documented in receipt schema `arks-personality-survey/v1` (≥10 main-category commands beyond IW-PERSONALITY-EXIT named set) |
| Remainder Partial×(117−expand) | Honest Partial with this SoT cite |
| Refuse | Claiming all 117 without per-row evidence |

### IW-DAEMON-FLAGS-PARITY detail

| Flag / service | Parity bar |
|----------------|------------|
| `--verbose` | Log detail on connection/request when set |
| `--timeout` / `--init-timeout` | Idle / init timers enforced (Docker evidence) |
| `--max-connections` | Accept-then-drop / refuse over limit |
| `--strict-paths` | Exact path match; no `.git` alias when set |
| `--base-path` + dirs | Remap + allow-list |
| `--reuseaddr` | Flag honored (Unix SO_REUSEADDR Partial OK if documented) |
| `--listen` / `--port` | Bind host/port as requested |
| `--enable`/`--disable` + upload-archive / receive-pack | Service gate matches peer literals |
| `related:git:// client` | Pointer only — client lane Claimed elsewhere; do not double-count |

**Terminal Refuse (unchanged):** classic SysV double-fork; inetd; privilege drop; templates; access-hook; invent Parity from clap parse alone.

### Docker / compose

| Flag / service | Role |
|----------------|------|
| `dod-hub-matrix` + `ARKS_HUB_IW_ACTIONS=1` | Actions metadata Claimed |
| `dod-hub-matrix` + `ARKS_HUB_IW_ASSETS=1` | Assets Claimed |
| `dod-hub-matrix` + `ARKS_HUB_IW_CHECKOUT=1` | Checkout resolve Claimed |
| `dod-hub-matrix` + `ARKS_HUB_IW_UX=1` | Config/alias/completion Claimed |
| `dod-hub-matrix` + `ARKS_HUB_IW_STUBS=1` | Stub expand Claimed |
| `dod-cli-matrix` + `ARKS_PERSONALITY_SURVEY=1` | Survey expand Claimed |
| `dod-daemon-matrix` + `ARKS_DAEMON_FLAGS_PARITY=1` | Flag Parity Claimed |

```bash
docker compose -f verify/docker-compose.yml run --rm -e ARKS_DOD_STRICT=1 -e ARKS_HUB_IW_ACTIONS=1 dod-hub-matrix
docker compose -f verify/docker-compose.yml run --rm -e ARKS_DOD_STRICT=1 -e ARKS_HUB_IW_ASSETS=1 dod-hub-matrix
docker compose -f verify/docker-compose.yml run --rm -e ARKS_DOD_STRICT=1 -e ARKS_HUB_IW_CHECKOUT=1 dod-hub-matrix
docker compose -f verify/docker-compose.yml run --rm -e ARKS_DOD_STRICT=1 -e ARKS_HUB_IW_UX=1 dod-hub-matrix
docker compose -f verify/docker-compose.yml run --rm -e ARKS_DOD_STRICT=1 -e ARKS_HUB_IW_STUBS=1 dod-hub-matrix
docker compose -f verify/docker-compose.yml run --rm -e ARKS_DOD_STRICT=1 -e ARKS_PERSONALITY_SURVEY=1 dod-cli-matrix
docker compose -f verify/docker-compose.yml run --rm -e ARKS_DOD_STRICT=1 -e ARKS_DAEMON_FLAGS_PARITY=1 dod-daemon-matrix
```

### Spec-first commit contract

1. Land this SoT (+ research) **before** product/harness deepen.
2. Commit body cites `research:root/20261111123556` and `sot:root/arks-in-scope-leftovers-wave`.
3. Claimed / Parity flips **only** with scrubbed evidence + residual register update.
4. Stop after one classified mismatch — no Docker thrash.
5. Out-of-scope rows: document only — no invent Claimed work.

### Cross-links

| Ledger | Role |
|--------|------|
| [[sot:root/arks-honesty-leftovers-wave]] | Prior mutate / exit tables / B5+ Claimed |
| [[sot:root/verify-cleanroom-symmetry-matrix]] | Three pillars + related serve |
| [[sot:root/production-residual-register]] | Post-terminal next-queue |
| [[research:root/20261111123556]] | Web research + tip inventory |

## Why

Agents otherwise invent Claimed for OUT groups, treat later Deferred as v1 must-haves, reopen SysV double-fork / L9 / live secrets, or leave in-pillar Actions/assets/checkout/CLI Partial/daemon flags unscoped. One research-gated SoT freezes the Phase 0 in-vs-out table and Phase 1+ Claimed bars.

## Verification

1. Discovery README entry + residual register next-queue cite this SoT.
2. After Phase B: each in-scope ID is Claimed with evidence **or** honest Partial/Refuse with cite; OUT list untouched.
3. No invent PASS; SysV double-fork / L9 / G8 secrets / Impossible remain terminal OUT.
