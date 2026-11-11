---
id: sot:root/arks-remaining-incompletes-wave
title: Remaining incompletes wave — hub full-matrix / personality expand / daemon Refuse / docs
status: active
created: 2026-11-11T01:59:24Z
updated: 2026-11-11T02:05:15Z
tags: [design, residual, hub-full-matrix, personality, daemon, b5-plus, docs-honesty, no-invent-claimed, docker, spec-first]
verify: manual
rel:
  depends_on:
    - research:root/20261111105924
    - research:root/20261111093543
    - sot:root/arks-verify-docker-e2e-claim-wave
    - sot:root/arks-partial-residual-wave
    - sot:root/arks-remaining-implement-wave
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
    - verify/matrices/hub-gh-arks-e2e-matrix.csv
---

# Remaining incompletes wave — design SoT

Document revision **0.1.1** (SoT edit counter only — **not** a product/crate release). English-only.

**This SoT gates leftovers after [[sot:root/arks-verify-docker-e2e-claim-wave]].** Tip surveyed **`7b1f216`**. **Design ≠ Claimed.** Cite [[research:root/20261111105924]] + this SoT before product Claimed flips.

## Rule

### Absolute honesty

1. Partial stubs, CSV edits alone, ExitCodePolicy units, B3 remote, and sample drop-in receipts **MUST NOT** invent full-matrix or full-personality Claimed.
2. Product paths **MUST NOT** spawn `git` / `gh` / `ssh` / `ollama`. Verify **MAY** compare pin peers.
3. Clean-room Linux Docker (`arks-verify-m1` + Git pin **2.39.5**) is normative DoD for Claimed / heavy verify.
4. New/extended slots follow [[sot:root/verify-directory-conventions]] §7 — Spec first, then harness.
5. Prefer honest **Partial** / **Refuse** / **later** over invent Claimed. **No origin push.**

### Ordered backlog

| Order | ID | Surface | Status now | Close bar | Refuse / non-goal | Docker / verify |
|------:|----|---------|------------|-----------|-------------------|-----------------|
| 1 | **IW-HUB-FULL** | Hub v1 CSV Missing→0 + expand Claimed | **Claimed** ([[evidence:root/20261111020350-hub-full-expand]]; v1 Missing=0; Partial leftovers honest) | (a) v1 **Missing=0**; (b) expanded Claimed subset with Docker receipt | Invent Claimed for stub exit-2 | `dod-hub-matrix` + `ARKS_HUB_IW_FULL=1` |
| 2 | **IW-PERSONALITY-FULL** | Personality drop-in beyond sample | **Claimed** expand ([[evidence:root/20261111020515-personality-expand]]); per-cmd tables Partial | Expanded Docker sample + receipt; cli survey Missing=0 | Claim every CLI Partial row as drop-in | `dod-cli-matrix` + `ARKS_PERSONALITY_DROPIN=1` |
| 3 | **IW-DAEMON-FLAGS / B5+** | Daemon deepen + facility | **Partial** / **Refuse (terminal)** | Documented; no collector → no B5+ Claimed | Force facility Claimed without collector | honesty only |
| 4 | **DOCS-HONESTY** | README / docs vs Claimed register | **Synced** this wave | Public copy cites Claimed receipts | Invent beyond receipts | Manual |

### IW-HUB-FULL detail

#### A — Inventory Missing→0 (mandatory)

Every **phase=v1** row in `verify/matrices/hub-gh-arks-e2e-matrix.csv` MUST leave `status_now=Missing`. Allowed dispositions:

| Disposition | When |
|-------------|------|
| **Claimed** | Peer-faithful happy path + Docker receipt (subset or expand list) |
| **Partial** | REST-wired against local forge **or** discoverable fail-closed stub (exit 2 + honesty; never silent ignore) |
| **Refuse/N/A** | Explicit non-goal (spawn git/gh required; OUT groups) |
| **later** | Spec amend only — demote from v1 when not a must-have for local-forge symmetry |

**Tip classification (normative starting map):**

| Class | Leaves (examples) | Target |
|-------|-------------------|--------|
| Already Claimed (flat-10) | auth login/status; repo create/list; issue create/list/view; pr create/list/view | Keep Claimed |
| REST-wired Partial (CSV lag) | label list/create; release list/create; run list/create; workflow list; org list; search *; auth logout/token; repo view; api; status | Missing→**Partial** (or Claimed if expand receipt) |
| Fail-closed stub Partial | pr close/reopen/edit/comment/merge/…; issue close/…; release view/edit/…; repo edit/delete/…; config/alias/completion stubs | Missing→**Partial** with honesty note |
| Expand Claimed candidates | auth logout/token; repo view; label list/create; release list/create; org list; search issues/prs/repos | Claimed **only** with `ARKS_HUB_IW_FULL=1` Docker receipt |
| Stay Partial (not Claimed this wave) | Actions run watch/cancel/rerun/download; workflow run/enable/disable; pr checkout worktree; release upload/download assets | Partial or later — **no invent CI Claimed** |

#### B — Expanded Claimed subset (`ARKS_HUB_IW_FULL=1`)

**Normative expand list (this wave):**

`auth logout`, `auth token`, `repo view`, `label list`, `label create`, `release list`, `release create`, `org list`, `search issues`, `search prs`, `search repos`.

Plus prior flat-10 remain Claimed. Harness MUST:

1. Start local forge under `arks serve` (or env API).
2. Exercise expand leaves with peer-faithful JSON/stdout checks.
3. Write scrubbed `.spec/evidence/YYYYMMDDHHmmss-hub-full-expand/` + `receipt.json`.
4. Fail closed under `ARKS_DOD_STRICT=1`.

Full-matrix STRICT without expand flag: **Missing=0** required; Partial rows OK (honesty).

**Refuse as Claimed:** B3 alone; CSV edit alone; help-only greens.

### IW-PERSONALITY-FULL detail

| Scenario | Bar |
|----------|-----|
| **PD-GIT expand** | Prior sample (`--version` / bad-flag) **plus** at least one usage-class (`-h`/`--help` exit 0) and optional porcelain-class probe when SoT names it — compare exit **class** to pin `git` |
| **PD-GH expand** | Prior sample **plus** `issue --help` exit 0 and one happy `issue list` (or `auth status`) against local forge under `ARKS_PERSONALITY=gh` |
| **Per-cmd tables** | Remain **Partial** until row-level peer tables + evidence — expand sample ≠ full CLI Claimed |
| **cli-git-help Missing** | Already **0** — do not reopen as Missing |

Receipt: `.spec/evidence/YYYYMMDDHHmmss-personality-expand/` (or amend drop-in receipt schema `arks-personality-dropin/v2` with expand checks).

**Refuse as Claimed:** ExitCodePolicy unit greens alone; unified OR under `gh`/`git` personality name.

### IW-DAEMON-FLAGS / B5+ (terminal)

| Item | Disposition |
|------|-------------|
| path-filter / `--syslog` openlog / `--user`/`--group` / detach | **Partial** (deepen closed) |
| Syslog facility/priority Claimed | **Refuse (terminal)** — no collector receipt ([[research:root/20261111105924]]) |
| Classic SysV double-fork Claimed | **Refuse (terminal)** — peer is single-fork+`setsid` |
| Windows Event Log as syslog | **N/A** / Missing refuse (unchanged) |
| Archive byte-identical | **Claimed** elsewhere (IW-B5) — do not reopen |

Reopen B5+ **only** with deliberate collector Docker receipt + SoT amend.

### DOCS-HONESTY

Public `README.md` / `docs/completion.md` / `docs/self-host.md` / `docs/arks-hub.md` / `docs/daemon.md` MUST NOT lag behind residual Claimed:

- IW-FORGE-SERVE Claimed ([[evidence:root/20261111011425-self-host-forge-e2e]])
- IW-HUB-FLAT subset Claimed ([[evidence:root/20261111011539-hub-flat-subset]]); full v1 Missing closed per this SoT
- IW-PERSONALITY sample (+ expand when evidenced)
- B5 archive Claimed; B5+ Refuse; B7 HTTP+SSH+forge Claimed

### Docker / compose

| Flag / service | Role |
|----------------|------|
| `dod-hub-matrix` + `ARKS_HUB_IW_FULL=1` | Expand Claimed + Missing=0 check |
| `dod-hub-matrix` + `ARKS_DOD_STRICT=1` (no IW flag) | Fail if v1 Missing>0 |
| `dod-cli-matrix` + `ARKS_PERSONALITY_DROPIN=1` | Sample + expand personality |
| `dod-daemon-matrix` | Honesty only — no B5+ Claimed |

```bash
docker compose -f verify/docker-compose.yml run --rm -e ARKS_DOD_STRICT=1 -e ARKS_HUB_IW_FULL=1 dod-hub-matrix
docker compose -f verify/docker-compose.yml run --rm -e ARKS_DOD_STRICT=1 -e ARKS_PERSONALITY_DROPIN=1 dod-cli-matrix
```

### Spec-first commit contract

1. Land/amend this SoT (+ research) before product/harness deepen.
2. Commit body cites `research:root/20261111105924` and `sot:root/arks-remaining-incompletes-wave`.
3. Claimed flips **only** with scrubbed evidence + residual register update.
4. Stop after one classified mismatch — no Docker thrash.

### Cross-links

| Ledger | Role |
|--------|------|
| [[sot:root/arks-verify-docker-e2e-claim-wave]] | Prior Claimed hosting/flat/sample |
| [[sot:root/arks-partial-residual-wave]] | Parent Partial bars |
| [[sot:root/production-residual-register]] | Post-terminal next-queue |
| [[research:root/20261111105924]] | Web research receipt |

## Why

Agents otherwise invent full hub Claimed from CSV edits, treat personality sample as full CLI drop-in Done, reopen B5+ without collectors, or leave public docs advertising stale Partial. One research-gated SoT freezes Missing→0 honesty, expand Claimed bars, terminal Refuse, and docs sync order.

## Verification

1. `spec check` clean for this SoT + discovery README entry.
2. After Phase B: hub v1 Missing=0; expand Claimed only with evidence id; personality expand receipt or honest Partial; B5+ still Refuse; docs match register.
3. No invent PASS.
