---
id: sot:root/arks-partial-residual-wave
title: Partial residual wave — design SoT (research-gated)
status: active
created: 2026-11-11T00:35:43Z
updated: 2026-11-11T01:14:25Z
tags: [design, residual, partial, forge-in-serve, hub-flat, personality, daemon, hosting-e2e, no-invent-claimed, docker, spec-first]
verify: manual
rel:
    depends_on:
    - research:root/20261111093543
    - research:root/20261111090827
    - sot:root/arks-remaining-implement-wave
    - sot:root/arks-verify-docker-e2e-claim-wave
    - sot:root/arks-claimed-residual-gates
    - sot:root/production-residual-register
    - sot:root/verify-directory-conventions
    - sot:root/verify-cleanroom-symmetry-matrix
    - sot:root/arks-self-host-server
    - sot:root/arks-cli-personality-factory
    - sot:root/arks-cli-output-exit-contract
    - sot:root/arks-hub-cli-surface
    - sot:root/arks-hub-product-intent
    - sot:root/arks-daemon-product-intent
    - sot:root/product-verification-clean-room
    - sot:root/language-english-only
  governs:
    - verify/scripts/run-self-host-e2e.sh
---

# Partial residual wave — design SoT (research-gated)

Document revision **0.1.3** (SoT edit counter only — **not** a product/crate release). English-only.

**This SoT gates the four Partial leftovers** after [[sot:root/arks-remaining-implement-wave]] Claimed rows (IW-B5 / IW-B7-SSH / IW-B7-FORGE / IW-SSH-LABEL). Tip surveyed **`ff39b69`**. **Design ≠ Claimed.**

Product/crate changes for each row require:

1. This SoT row cited in the Spec-first commit / plan, **and**
2. Supporting research [[research:root/20261111093543]] (plus prior [[research:root/20261111090827]] where still applicable), **and**
3. Partial vs Claimed vs Refuse bar below understood (STRICT fail-closed; **no invent PASS**).

## Rule

### Absolute honesty

1. Partial smokes, SKIP harness greens, host PowerShell, B3 client remote Claimed, and ExitCodePolicy unit greens **MUST NOT** invent Claimed for these rows.
2. Product paths **MUST NOT** spawn `git` / `gh` / `ssh` / `ollama`.
3. Clean-room Linux Docker (`arks-verify-m1` + Git pin **2.39.5**) is normative DoD ([[sot:root/product-verification-clean-room]]).
4. New Claimed slots register per [[sot:root/verify-directory-conventions]] §7 — Spec first, then harness (`gNNN-<kebab>` / `dod-<kebab>` orthography).
5. This wave is **adjacent** to the three symmetry pillars — **not** a fourth pillar and **not** a reopen of Claimed residual B-gates already closed.

### Ordered implementation backlog (this wave only)

Order is **Spec → research cite → product/harness deepen → Docker/focused verify → SoT status amend**. Do not reorder to chase green SKIP. Prefer honest **Partial** / **Refuse** over invent Claimed.

| Order | ID | Surface | Status now | Partial bar (honest) | Claimed bar | Refuse / non-goal | Docker / verify |
|------:|----|---------|------------|----------------------|-------------|-------------------|-----------------|
| 1 | **IW-FORGE-SERVE** | Forge-in-serve hosting depth | **Claimed** ([[evidence:root/20261111011425-self-host-forge-e2e]]) | Default `arks serve` cohosts HTTP+SSH+forge; `dod-self-host` probes `--no-forge`/forge help markers; STRICT fail-closed | Hosting e2e: local forge module PR **and** issue host journeys under `dod-self-host` + scrubbed receipt; **B3 ≠ this row** | Invent Claimed from cohost smoke / B3 / hub-serve transitional | `dod-self-host` → `run-self-host-e2e.sh` |
| 2 | **IW-HUB-FLAT** | Hub flat-leaf depth | **Partial** (+ **B3 Claimed**; Spec honesty) | Flat leaves + hub serve forwards remain Partial stubs; B3 HTTPS remote happy path stays Claimed separately | Peer-faithful leaf bodies + hub matrix STRICT greens | Invent leaf Claimed from B3 or stub exit-2 greens | Symmetry `dod-hub-matrix` |
| 3 | **IW-PERSONALITY** | Personality / Registry / ExitCodePolicy | **Partial** (skeleton + OR; drop-in Missing) | Resolver + Registry + OverlapFacade + ExitCodePolicy OR-of-failure unit-tested | Peer-native stdout/exit drop-in evidence for `git`/`gh` personalities | Invent drop-in Claimed from ExitCodePolicy units alone; spawn stock peers | Symmetry `dod-cli-matrix` + personality SoT amend |
| 4 | **IW-DAEMON-FLAGS** | Daemon deepen (non-B5) | **Partial** / **B5+ Refuse** (terminal; deepen closed) | path-filter, `--syslog` openlog, `--user`/`--group`, detach Partial; help cites single-fork peer + Refuse bars | **No** Claimed for classic double-fork or syslog facility/priority; archive Claimed = IW-B5 only | B5+ facility Claimed; classic double-fork Claimed; Windows Event Log as syslog | `dod-daemon-matrix` / archive slot as applicable |

**Explicit non-items (do not schedule as implement this wave):** B5+ syslog facility/priority Claimed (**Refuse**); classic SysV double-fork Claimed (**Refuse** — peer detach is single-fork+`setsid`); B8 Windows SIGHUP (**N/A**); OUT `extension`/`copilot` graduation; product OpenSSH embed; L9 / live G8 invent PASS; full gh-matrix Claimed leaf bodies.

### Partial vs Claimed vs Refuse (per-class detail)

#### IW-FORGE-SERVE

- **Cite:** [[research:root/20261111093543]] (Forgejo cohost architecture; Githome/oompa hosting e2e patterns); owning [[sot:root/arks-self-host-server]].
- **Partial (met historically):** default cohost; help/cohost probe before host journeys.
- **Claimed (met):** Docker receipt [[evidence:root/20261111011425-self-host-forge-e2e]] — PR create/list + issue create/list against **in-process** forge under `arks serve` via `dod-self-host` (arks-only; **no** `gh` spawn). Playbook: [[sot:root/arks-verify-docker-e2e-claim-wave]].
- **Refuse as Claimed:** B3 client remote alone; `arks hub serve` transitional; help-only smoke.

#### IW-HUB-FLAT

- **Cite:** [[research:root/20261111093543]]; [[sot:root/arks-hub-cli-surface]] / [[sot:root/arks-hub-product-intent]].
- **Partial:** stub/Partial local REST bodies; flat argv discoverable; refuse unimplemented with non-zero.
- **Claimed (future):** peer-faithful stdout/JSON for surveyed v1 leaves + hub matrix STRICT.
- **Refuse as Claimed:** B3 over-read; treating host forge API smoke as client leaf Claimed.

#### IW-PERSONALITY

- **Cite:** [[research:root/20261111093543]] (`gh` exit-codes + scriptability); [[sot:root/arks-cli-personality-factory]]; [[sot:root/arks-cli-output-exit-contract]].
- **Partial (met):** PersonalityResolver + Registry + OverlapFacade + ExitCodePolicy **OR-of-failure** for unified `arks`.
- **Claimed (future):** `git` / `gh` personality drop-in — peer stdout shape + peer exit classes under alias/shim/`ARKS_PERSONALITY` with matrix receipt.
- **Refuse as Claimed:** unit OR greens alone; unified OR under `gh`/`git` personality.

#### IW-DAEMON-FLAGS / B5+

- **Cite:** [[research:root/20261111093543]] (git-daemon docs; `openlog`/`LOG_DAEMON`; single-fork `daemonize`); [[sot:root/arks-daemon-product-intent]]; [[sot:root/arks-claimed-residual-gates]] B5+.
- **Partial deepen:** **closed** this wave (flags already Partial).
- **Claimed:** none for this row; IW-B5 archive Claimed already closed separately.
- **Refuse (terminal):** syslog facility/priority Claimed without collector receipt; classic double-fork Claimed (not the pin peer).

### Implementation contract (Phase B)

Docker Claimed playbook for leftovers **1–3:** [[sot:root/arks-verify-docker-e2e-claim-wave]] (compose services, ports, scenarios, receipts, product-vs-harness; order hosting → hub STRICT subset → personality drop-in). B5+ stays Refuse there too.

1. **IW-FORGE-SERVE first:** deepen `run-self-host-e2e.sh` per Docker e2e SoT (SH-ISSUE + SH-PR); STRICT fail-closed; **do not** flip Claimed without Docker receipt.
2. **IW-HUB-FLAT:** peer-faithful Claimed-subset leaves + `dod-hub-matrix` subset STRICT (`ARKS_HUB_IW_FLAT=1`) — no invent from B3 or full-CSV Missing.
3. **IW-PERSONALITY:** drop-in sample under `dod-cli-matrix` (`ARKS_PERSONALITY_DROPIN=1`); ExitCodePolicy units ≠ Claimed.
4. **IW-DAEMON-FLAGS:** help/SoT honesty for single-fork peer vs classic double-fork Refuse; no new Claimed flags.

### Spec-first commit contract (mandatory)

Before any product PR/commit for an IW-* row in this SoT:

1. Spec amend (this SoT and/or owning product SoT) stating acceptance.
2. Research cite in commit body (`research:root/20261111093543`).
3. Harness names match table above / §7 kebab conventions.
4. Claimed status flips **only** with scrubbed `.spec/evidence/` + residual register update.

### Cross-links (normative)

| Ledger | Role |
|--------|------|
| [[sot:root/arks-remaining-implement-wave]] | Prior wave; Claimed IW-B5/B7/SSH-label; points Partial leftovers here |
| [[sot:root/arks-claimed-residual-gates]] | B1–B9 / B5+ Refuse |
| [[sot:root/production-residual-register]] | Post-terminal next-queue |
| [[sot:root/verify-directory-conventions]] | §7 + `gNNN`/`dod-*` naming (0.6.0) |
| [[sot:root/arks-self-host-server]] | Serve umbrella + hosting e2e honesty |
| [[sot:root/arks-verify-docker-e2e-claim-wave]] | Docker-under-verify Claimed path for IW-FORGE-SERVE / HUB-FLAT / PERSONALITY |
| [[research:root/20261111093543]] | Web research receipt for this wave |

## Why

Agents otherwise invent Claimed from B3, cohost smokes, ExitCodePolicy units, or openlog arming. A dedicated Partial-residual SoT freezes bars so deepen stays honest and Claimed waits for normative Docker receipts.

## Verification

1. `spec check` clean for this SoT + linked residual / remaining-wave / README discovery entries.
2. No product Claimed language added without matching evidence id.
3. Public docs MAY cite IW-* Partial/Refuse honestly — never as Done without receipts.
