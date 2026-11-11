---
id: sot:root/arks-honesty-leftovers-wave
title: Honesty leftovers wave — hub mutate / personality exit tables / B5+ collector
status: active
created: 2026-11-11T02:43:12Z
updated: 2026-11-11T02:52:07Z
tags: [design, residual, hub-mutate, personality-exit-tables, daemon, b5-plus, syslog-collector, no-invent-claimed, docker, spec-first]
verify: manual
rel:
  depends_on:
    - research:root/20261111114312
    - research:root/20261111105924
    - sot:root/arks-remaining-incompletes-wave
    - sot:root/arks-verify-docker-e2e-claim-wave
    - sot:root/arks-partial-residual-wave
    - sot:root/arks-claimed-residual-gates
    - sot:root/production-residual-register
    - sot:root/verify-directory-conventions
    - sot:root/verify-cleanroom-symmetry-matrix
    - sot:root/arks-hub-cli-surface
    - sot:root/arks-hub-gh-command-matrix
    - sot:root/arks-cli-personality-factory
    - sot:root/arks-cli-output-exit-contract
    - sot:root/arks-daemon-product-intent
    - sot:root/product-verification-clean-room
    - sot:root/language-english-only
    - sot:root/public-surface
  governs:
    - verify/scripts/run-hub-gh-arks-matrix-e2e.sh
    - verify/scripts/run-cli-git-arks-matrix-e2e.sh
    - verify/scripts/run-daemon-git-arks-matrix-e2e.sh
    - verify/matrices/hub-gh-arks-e2e-matrix.csv
---

# Honesty leftovers wave — design SoT

Document revision **0.1.1** (SoT edit counter only — **not** a product/crate release). English-only.

**This SoT gates leftovers after [[sot:root/arks-remaining-incompletes-wave]] tip `812d298`.** Phase B Claimed flips cite evidence below. Cite [[research:root/20261111114312]] + this SoT.

## Rule

### Absolute honesty

1. Stub exit-2, CSV edits alone, ExitCodePolicy unit greens, and `openlog` without collector PRI **MUST NOT** invent Claimed.
2. Product paths **MUST NOT** spawn `git` / `gh` / `ssh` / `ollama`. Verify **MAY** compare pin peers.
3. Clean-room Linux Docker (`arks-verify-m1` + Git pin **2.39.5**) is normative DoD for Claimed / heavy verify.
4. New/extended slots follow [[sot:root/verify-directory-conventions]] — Spec first, then harness.
5. Prefer honest **Partial** / **Refuse** over invent Claimed. **No origin push.**

### Ordered backlog

| Order | ID | Surface | Status now | Close bar | Refuse / non-goal | Docker / verify |
|------:|----|---------|------------|-------------------|-----------------|-----------------|
| 1 | **IW-HUB-MUTATE** | Non-expand v1 mutate/view subset | **Claimed** ([[evidence:root/20261111025142-hub-mutate-subset]]; v1 Partial leftovers remain) | Peer-faithful happy paths + Docker receipt for SoT expand list | Invent Claimed for stub exit-2; Actions/assets/checkout | `dod-hub-matrix` + `ARKS_HUB_IW_MUTATE=1` |
| 2 | **IW-PERSONALITY-EXIT** | Per-command peer numeric exit tables | **Claimed** named rows ([[evidence:root/20261111025151-personality-exit-tables]]); CLI Partial×117 remain | Named SoT rows + Docker numeric peer compare | Claim all 117 CLI Partial rows | `dod-cli-matrix` + `ARKS_PERSONALITY_EXIT_TABLES=1` |
| 3 | **IW-B5-PLUS** | Syslog facility/priority | **Claimed** ([[evidence:root/20261111025206-daemon-b5-plus-syslog]]; PRI=30 facility=3 severity=6) | Collector receipt: facility=`daemon`(3) + severity evidenced | Force Claimed from openlog alone | `dod-daemon-matrix` + `ARKS_DAEMON_B5_PLUS=1` (collector) |
| 4 | **DOCS-REGISTER** | Residual + product SoT honesty | **Synced** this wave | Sync statuses + evidence ids | Invent beyond receipts | Manual |

### IW-HUB-MUTATE detail

#### Normative Claimed expand list (this wave)

| Leaf | Peer-faithful bar |
|------|-------------------|
| `issue close` | PATCH state=closed; view shows closed |
| `issue reopen` | PATCH state=open |
| `issue edit` | PATCH title and/or body |
| `issue comment` | POST comment; response includes body |
| `pr close` | PATCH state=closed |
| `pr reopen` | PATCH state=open |
| `pr edit` | PATCH title and/or body |
| `pr comment` | POST comment (issues comments API) |
| `pr merge` | Local-forge **metadata** merge (`merged`/`closed`); **≠** git worktree merge; never spawn git |
| `release view` | GET release by tag or id; JSON includes `tag_name` |
| `label edit` | PATCH/replace name|color|description |
| `label delete` | DELETE label; subsequent list omits it |

Harness MUST:

1. Start local forge under `arks serve` (or env API).
2. Seed create leaves, exercise mutate list, assert peer-faithful JSON/stdout.
3. Write scrubbed `.spec/evidence/YYYYMMDDHHmmss-hub-mutate-subset/` + `receipt.json`.
4. Fail closed under `ARKS_DOD_STRICT=1`.

#### Stay Partial (explicit)

| Class | Leaves (examples) |
|-------|-------------------|
| Actions execute | `run watch|cancel|rerun|download|delete|view`; `workflow run|enable|disable|view` |
| Release assets | `release upload|download|delete-asset|edit|delete` |
| Checkout | `pr checkout`, `co` (resolve/print only; must not spawn git) |
| Config / alias / completion | installer / profile mutation depth |
| Auth refresh | OAuth refresh not Claimed |
| Group-help-only | bare `pr`/`issue`/`repo`/… group rows |
| Other stubs | `issue lock|pin|transfer|…`; `repo edit|delete|fork|…`; `status`/`api` passthrough depth beyond Partial |

**Refuse as Claimed:** B3 alone; CSV edit alone; help-only greens; invent CI runner.

### IW-PERSONALITY-EXIT detail

#### Named Claimed exit-table rows (this wave)

| Row id | Personality | Scenario | Expected exit |
|--------|-------------|----------|---------------|
| **XT-GIT-VER** | `git` | `--version` | `0` (peer pin class) |
| **XT-GIT-BAD** | `git` | unknown flag | non-zero (pin class; typically `1`/`129`-class — match **class**, record numeric) |
| **XT-GIT-HELP** | `git` | `--help` | `0` |
| **XT-GH-HELP** | `gh` | `pr --help` | `0` |
| **XT-GH-UNK** | `gh` | unknown leaf | `2` (usage/stub class per contract) |
| **XT-GH-OK** | `gh` | `pr list` happy vs local forge | `0` |
| **XT-GH-AUTH** | `gh` | auth-required path when token absent (hub auth status / equivalent) | `4` **or** honest Partial if product cannot peer `gh` auth=4 yet — **do not invent 4** |

Receipt schema `arks-personality-exit-tables/v1` under `.spec/evidence/YYYYMMDDHHmmss-personality-exit-tables/`.

**Refuse as Claimed:** ExitCodePolicy unit greens alone; unified OR under wrong personality; inventing `4` without product path.

Remaining CLI survey Partial×117 stay Partial.

### IW-B5-PLUS (collector reopen or terminal Refuse)

| Item | Disposition rule |
|------|------------------|
| `openlog(LOG_DAEMON)` alone | **Partial** — insufficient |
| `syslog(priority, …)` + collector PRI decode | **Claimed** only with Docker receipt proving facility **daemon (3)** and documented severity (e.g. `LOG_INFO` → severity 6; PRI=30) |
| No collector / PRI mismatch / Windows Event Log | **Refuse (terminal)** retained; document attempt |
| Classic SysV double-fork | **Refuse (terminal)** unchanged — peer is single-fork+`setsid` |
| Archive byte-identical | **Claimed** elsewhere (IW-B5) — do not reopen |

Collector harness (`ARKS_DAEMON_B5_PLUS=1`):

1. Start UDP/TCP or unix-socket syslog collector in clean-room (rsyslog or equivalent in-script listener).
2. Run `arks daemon --syslog` (minimal listen) and force one INFO syslog line with known marker.
3. Parse collector capture for marker + PRI/facility/severity.
4. Receipt `.spec/evidence/YYYYMMDDHHmmss-daemon-b5-plus-syslog/` — Claimed only on match.

If STRICT collector path cannot green honestly: leave **Refuse**, write honesty note in receipt or SoT amend — **never invent Claimed**.

### DOCS-REGISTER

Update [[sot:root/production-residual-register]] next-queue + owning product SoTs ([[sot:root/arks-hub-cli-surface]], [[sot:root/arks-cli-personality-factory]], [[sot:root/arks-cli-output-exit-contract]], [[sot:root/arks-daemon-product-intent]], [[sot:root/arks-claimed-residual-gates]]) to match final Claimed/Partial/Refuse + evidence ids. Public docs only when receipts exist.

### Docker / compose

| Flag / service | Role |
|----------------|------|
| `dod-hub-matrix` + `ARKS_HUB_IW_MUTATE=1` | Mutate subset Claimed |
| `dod-cli-matrix` + `ARKS_PERSONALITY_EXIT_TABLES=1` | Named exit-table Claimed |
| `dod-daemon-matrix` + `ARKS_DAEMON_B5_PLUS=1` | Collector B5+ attempt |

```bash
docker compose -f verify/docker-compose.yml run --rm -e ARKS_DOD_STRICT=1 -e ARKS_HUB_IW_MUTATE=1 dod-hub-matrix
docker compose -f verify/docker-compose.yml run --rm -e ARKS_DOD_STRICT=1 -e ARKS_PERSONALITY_EXIT_TABLES=1 dod-cli-matrix
docker compose -f verify/docker-compose.yml run --rm -e ARKS_DOD_STRICT=1 -e ARKS_DAEMON_B5_PLUS=1 dod-daemon-matrix
```

### Spec-first commit contract

1. Land this SoT (+ research) before product/harness deepen.
2. Commit body cites `research:root/20261111114312` and `sot:root/arks-honesty-leftovers-wave`.
3. Claimed flips **only** with scrubbed evidence + residual register update.
4. Stop after one classified mismatch — no Docker thrash.

### Cross-links

| Ledger | Role |
|--------|------|
| [[sot:root/arks-remaining-incompletes-wave]] | Prior Missing→0 + expand Claimed |
| [[sot:root/arks-claimed-residual-gates]] | B5+ historical Refuse; reopen gated here |
| [[sot:root/production-residual-register]] | Post-terminal next-queue |
| [[research:root/20261111114312]] | Web research receipt |

## Why

Agents otherwise invent hub Claimed from stubs, treat ExitCodePolicy units as per-cmd table Done, or reopen B5+ without a collector PRI path. One research-gated SoT freezes mutate Claimed bars, named exit-table rows, collector reopen criteria, and Stay Partial / Refuse honesty.

## Verification

1. `spec check` clean for this SoT + discovery README entry.
2. After Phase B: mutate Claimed only with evidence; exit-table Claimed only for named rows; B5+ Claimed only with collector PRI else Refuse; register synced.
3. No invent PASS.
