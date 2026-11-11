---
id: sot:root/arks-verify-docker-e2e-claim-wave
title: Verify Docker e2e claim wave — hosting / hub STRICT / personality drop-in
status: active
created: 2026-11-11T01:11:52Z
updated: 2026-11-11T01:11:52Z
tags: [design, verify, docker, e2e, hosting, forge-in-serve, hub-flat, personality, no-invent-claimed, spec-first, clean-room]
verify: manual
rel:
  depends_on:
    - research:root/20261111093543
    - research:root/20261111090827
    - sot:root/arks-partial-residual-wave
    - sot:root/arks-remaining-implement-wave
    - sot:root/arks-claimed-residual-gates
    - sot:root/production-residual-register
    - sot:root/verify-directory-conventions
    - sot:root/verify-cleanroom-symmetry-matrix
    - sot:root/arks-self-host-server
    - sot:root/arks-cli-personality-factory
    - sot:root/arks-cli-output-exit-contract
    - sot:root/arks-hub-cli-surface
    - sot:root/arks-hub-product-intent
    - sot:root/product-verification-clean-room
    - sot:root/language-english-only
  governs:
    - verify/scripts/run-self-host-e2e.sh
    - verify/scripts/run-hub-gh-arks-matrix-e2e.sh
    - verify/scripts/run-cli-git-arks-matrix-e2e.sh
    - verify/docker-compose.yml
---

# Verify Docker e2e claim wave — hosting / hub STRICT / personality drop-in

Document revision **0.1.0** (SoT edit counter only — **not** a product/crate release). English-only.

**Design ≠ Claimed.** This SoT freezes the **Docker-under-`verify/`** path to resolve remaining Partial Claimed blockers from [[sot:root/arks-partial-residual-wave]] with the **same pattern** as prior Claimed residual gates: compose service → script → scrubbed `.spec/evidence/` receipt → status flip. Cite [[research:root/20261111093543]] (+ [[research:root/20261111090827]] where still applicable). **No new web research** required for this wave — existing notes cover Forgejo/Githome hosting e2e, hub vs B3 split, `gh` exit classes, and clean-room pin.

B5+ facility/priority and classic double-fork stay **Refuse** ([[sot:root/arks-claimed-residual-gates]]) unless SoT + collector deliberately reopen — **do not force**.

## Rule

### Absolute honesty

1. Clean-room Linux Docker only for Claimed / heavy verify: compose project `arks-verify-m1`, image **`arks-verify-m1:0.1.0`**, Git pin **2.39.5** ([[sot:root/product-verification-clean-room]]).
2. Product paths **MUST NOT** spawn `git` / `gh` / `ssh` / `ollama`. Verify harness **MAY** invoke pin `git` / stock peers **for comparison only**.
3. **No invent Claimed** from Partial smokes, SKIP harness greens, host PowerShell, B3 client remote alone, or ExitCodePolicy unit greens.
4. New/extended slots follow [[sot:root/verify-directory-conventions]] **0.6.0** — `dod-<concern>` / `dod-claimed-<concern>` kebab; script-first (no empty `gates/dod-*`).
5. **No origin push** from this wave. Spec-first commits before product/harness deepen.

### Explicit non-equations (normative)

| Must not confuse | Why |
|------------------|-----|
| **B3 ≠ hosting e2e (IW-FORGE-SERVE)** | B3 = hub **client** HTTPS remote happy path ([[evidence:root/20261110233655-claimed-hub-https]]). Hosting e2e = **in-process forge host** PR **and** issue create/list under `arks serve`. |
| **Unit ExitCodePolicy ≠ personality drop-in Claimed (IW-PERSONALITY)** | OR-of-failure unit greens prove skeleton policy only. Drop-in Claimed needs peer stdout + peer exit classes under `ARKS_PERSONALITY=git\|gh` (or argv0 shim) with matrix receipt. |
| **Hub stub / B3 ≠ flat-leaf Claimed (IW-HUB-FLAT)** | Discoverability + B3 remote ≠ peer-faithful flat leaf bodies. |
| **Cohost help smoke ≠ hosting e2e Claimed** | `--no-forge` / forge help markers are Partial only. |
| **B5+ / double-fork** | Terminal **Refuse** this wave. |

### Ordered backlog (this wave only)

| Order | ID | Compose service | Script | Status now | Claimed bar (this SoT) |
|------:|----|-----------------|--------|------------|------------------------|
| 1 | **IW-FORGE-SERVE** | `dod-self-host` | `verify/scripts/run-self-host-e2e.sh` | Partial (cohost probe) | Docker receipt: `arks serve` forge module + **issue create/list** + **PR create/list** (arks hub or flat client; **no** `gh` spawn) |
| 2 | **IW-HUB-FLAT** | `dod-hub-matrix` | `verify/scripts/run-hub-gh-arks-matrix-e2e.sh` | Partial (+ B3 Claimed) | Peer-faithful bodies for **Claimed subset** leaves + STRICT greens for that subset (receipt); full-matrix Missing ≠ invent PASS |
| 3 | **IW-PERSONALITY** | `dod-cli-matrix` | `verify/scripts/run-cli-git-arks-matrix-e2e.sh` (+ personality scenario) | Partial (skeleton + OR) | Drop-in sample under `ARKS_PERSONALITY=git` **and** `gh`: peer stdout shape + peer exit classes with scrubbed receipt |
| — | **IW-DAEMON-FLAGS / B5+** | `dod-daemon-matrix` / archive | existing | Partial / **Refuse** | **Out of Claimed scope** — do not schedule |

Do **not** reorder to chase green SKIP. Prefer honest **Partial** / **Refuse** over invent Claimed.

### Clean-room pin + ports

| Item | Normative | Demo / unprivileged (labeled) |
|------|-----------|-------------------------------|
| Image | `arks-verify-m1:0.1.0` | — |
| Git pin | **2.39.5** (oracle compare only) | Host Git **not** authority |
| Forge API | **3920** (arks-local) | Ephemeral `127.0.0.1:0` bind in harness OK |
| Git HTTP | **80** | **8080** demo |
| Git SSH | **22** | **2222** demo |
| `git://` daemon | **9418** | — |

Hosting e2e Claimed **MAY** use forge-only or HTTP+forge under `arks serve`; SSH module **not** required for IW-FORGE-SERVE Claimed (SSH wire already Claimed elsewhere). Prefer `--forge-listen 127.0.0.1:<ephemeral>` for isolation.

### Compose / scenario matrix

#### IW-FORGE-SERVE — `dod-self-host`

**Invocation:** `docker compose -f verify/docker-compose.yml run --rm -e ARKS_DOD_STRICT=1 dod-self-host`

| Scenario | Harness steps | Product must exist | Pass | Fail | Partial | Refuse as Claimed |
|----------|---------------|--------------------|------|------|---------|-------------------|
| **SH-COHOST** | `arks serve --help` shows forge / `--no-forge` | Help strings | Smoke green | Help regression | Default Partial deepen | Alone |
| **SH-ISSUE** | Start `arks serve` with forge; `arks hub auth login`; ensure repo; `issue create` + `issue list` against local forge | In-process forge POST/GET `/repos/.../issues`; hub/flat client | HTTP 2xx + JSON number/title; exit 0 | Bind/auth/API fail under STRICT | Journey Missing → STRICT exit 1 | B3 alone; curl-only without arks client |
| **SH-PR** | Same serve; `pr create` + `pr list` | In-process forge POST/GET `/repos/.../pulls` | Same | Same | Same | Same |
| **SH-RECEIPT** | Write under `/.dockerenv` only | — | Scrubbed dir + `receipt.json` | Host venue writes Claimed | No receipt → no Claimed flip | Invent from host PowerShell |

**Receipt path:** `.spec/evidence/YYYYMMDDHHmmss-self-host-forge-e2e/` with `receipt.json`, scrubbed server logs, issue/pr stdout snippets (no tokens).

**Schema (receipt):** `arks-self-host-forge-e2e/v1`; `gate: IW-FORGE-SERVE`; `venue: docker-dod-self-host`; checks list SH-ISSUE + SH-PR (+ SH-COHOST).

#### IW-HUB-FLAT — `dod-hub-matrix`

**Invocation:** `docker compose -f verify/docker-compose.yml run --rm -e ARKS_DOD_STRICT=1 -e ARKS_HUB_IW_FLAT=1 dod-hub-matrix`

| Scenario | Harness steps | Product must exist | Pass | Fail | Partial | Refuse as Claimed |
|----------|---------------|--------------------|------|------|---------|-------------------|
| **HF-DISC** | `arks hub --help` + flat `arks pr`/`arks issue` discoverable | Registry flat forge dispatch | Help green | Unknown leaf under STRICT | — | — |
| **HF-SUBSET** | Against local forge (script-started or env `ARKS_HUB_API`): Claimed-subset leaves happy path | Peer-faithful stdout/JSON for subset | Exit 0 + shape checks | Non-zero / garbage stdout | Bodies still stub-shaped | B3 over-read; invent full CSV Claimed |
| **HF-STRICT** | `ARKS_DOD_STRICT=1` + `ARKS_HUB_IW_FLAT=1` | Subset Missing=0; no `unknown command` | STRICT exit 0 for subset | Missing>0 in subset | Full CSV Missing may remain | Treating full-matrix SKIP as Claimed |

**Claimed subset (normative this wave — not full gh matrix):**

`auth login`, `auth status`, `repo create`, `repo list`, `issue create`, `issue list`, `issue view`, `pr create`, `pr list`, `pr view`.

Other CSV `v1` **Missing** rows stay Missing or may be rephased to `later` by Spec amend only — **MUST NOT** invent Claimed bodies. Full `dod-hub-matrix` without `ARKS_HUB_IW_FLAT=1` keeps existing STRICT fail-closed on all v1 Missing (honesty).

**Receipt path:** `.spec/evidence/YYYYMMDDHHmmss-hub-flat-subset/` + `receipt.json`.

#### IW-PERSONALITY — `dod-cli-matrix` (+ personality drop-in scenario)

**Invocation:** `docker compose -f verify/docker-compose.yml run --rm -e ARKS_DOD_STRICT=1 -e ARKS_PERSONALITY_DROPIN=1 dod-cli-matrix`

| Scenario | Harness steps | Product must exist | Pass | Fail | Partial | Refuse as Claimed |
|----------|---------------|--------------------|------|------|---------|-------------------|
| **PD-GIT** | `ARKS_PERSONALITY=git` sample cmds; compare exit class (+ stdout class) to pin `git` where peer is oracle | Personality resolve + git peer exit/stdout for sample | Match within SoT sample | Divergence under STRICT | Skeleton only | ExitCodePolicy unit alone |
| **PD-GH** | `ARKS_PERSONALITY=gh` sample forge leaves; exit classes cite `gh` 0/1/2/4 | Gh personality dispatch + peer exit classes | Match sample | Same | Same | Unified `arks` OR under gh name |
| **PD-RECEIPT** | Docker-only scrubbed compare logs | — | Receipt | Host invent | — | Spawn stock peers from **product** |

**Minimal Claimed sample (this wave):**

| Personality | Sample | Peer bar |
|-------------|--------|----------|
| `git` | `--version` / `version` (or documented alias); bad-flag → usage-class exit | Pin `git` exit class + version-ish stdout |
| `gh` | `pr --help` / unknown leaf → usage-class (2); one happy `pr list` against local forge when API up | `gh` exit-code classes ([[research:root/20261111093543]]); no invent unified OR |

Full CLI CSV Missing=0 remains a **separate** pillar DoD — personality drop-in Claimed **does not** require Missing=0.

**Receipt path:** `.spec/evidence/YYYYMMDDHHmmss-personality-dropin/` + `receipt.json`.

### Pass / Fail / Partial / Refuse (summary)

| Disposition | When |
|-------------|------|
| **Pass (Claimed)** | Docker venue + scenario greens + scrubbed evidence id registered in residual register + owning SoT status amend |
| **Fail** | `ARKS_DOD_STRICT=1` and Claimed bar not met (exit ≠ 0) |
| **Partial** | Scaffold/smoke/cohost/subset incomplete; default non-STRICT may exit 0 with honest SKIP/Partial language |
| **Refuse** | B5+ facility; classic double-fork; invent from B3 / units / help-only; product `git`/`gh` spawn |

### Product vs harness

| Concern | Product (crates) | Harness only (`verify/`) |
|---------|------------------|---------------------------|
| Forge issue/PR REST under `arks serve` | **Must exist** (already Partial in `arks-hub` serve) | Start/stop serve; drive journeys; receipt |
| Hub/flat client create/list/view | **Must exist** | STRICT subset walk; shape asserts |
| Peer-faithful stdout deepen | Only if harness proves gap | Prefer harness asserts first; minimal crate deepen |
| Personality drop-in stdout/exit | Resolver + peer exit mapping; deepen only if sample fails | Pin `git` compare; receipt |
| Compose service names | — | Keep `dod-self-host` / `dod-hub-matrix` / `dod-cli-matrix` (no fourth pillar) |

Optional future §7 register: `dod-claimed-forge-host` **only** if `dod-self-host` must stay scaffold forever — **prefer deepen `dod-self-host`** first (this SoT).

### Implementation contract (Phase B)

1. **Spec-first:** this SoT (+ cross-links) committed before harness/product deepen.
2. **IW-FORGE-SERVE:** deepen `run-self-host-e2e.sh` to SH-ISSUE + SH-PR; STRICT fail-closed; Claimed only with Docker receipt.
3. **IW-HUB-FLAT:** extend hub matrix for `ARKS_HUB_IW_FLAT=1` subset STRICT + peer-faithful asserts; minimal leaf deepen if harness gap; receipt.
4. **IW-PERSONALITY:** extend CLI matrix / sibling scenario for `ARKS_PERSONALITY_DROPIN=1`; Claimed only with dual-personality receipt; else honest Partial.
5. After each Claimed flip: amend [[sot:root/arks-partial-residual-wave]], [[sot:root/production-residual-register]], owning product SoTs; Spec sync commit.
6. Stop after one classified mismatch — **no Docker thrash loops**.

### Spec-first commit contract

1. Land/amend this SoT before product PR for any row above.
2. Commit body cites `research:root/20261111093543` and `sot:root/arks-verify-docker-e2e-claim-wave`.
3. Harness names match table / §7.
4. Claimed status flips **only** with scrubbed evidence + register update.

### Cross-links

| Ledger | Role |
|--------|------|
| [[sot:root/arks-partial-residual-wave]] | Parent Partial bars; this SoT = Docker Claimed path |
| [[sot:root/arks-self-host-server]] | Serve umbrella + port policy |
| [[sot:root/verify-cleanroom-symmetry-matrix]] | Pillars; hosting adjacent |
| [[sot:root/verify-directory-conventions]] | §7 naming 0.6.0 |
| [[research:root/20261111093543]] | Web research receipt (no new note) |

## Why

Partial residual SoT froze honesty bars; agents still need a **single Docker-e2e Claimed playbook** (services, ports, scenarios, receipts, product-vs-harness, order) so Phase B does not invent Claimed from B3, cohost smoke, or ExitCodePolicy units, and does not reopen B5+.

## Verification

1. `spec check` clean for this SoT + discovery README entry.
2. No Claimed language without evidence id.
3. Compose services named herein exist under `verify/docker-compose.yml`.
