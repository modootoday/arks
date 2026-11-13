---
id: sot:root/arks-native-e2e-checklist
title: ARKS-native e2e coverage checklist
status: active
created: 2026-11-11T05:20:00Z
updated: 2026-11-13T10:41:23Z
tags: [arks-native, e2e, docker, serve, self-host, lease, personality, daemon, agent, hub, celir, no-invent-claimed, behavioral-dod]
verify: manual
rel:
  depends_on:
    - sot:root/arks-self-host-server
    - sot:root/arks-claimed-residual-gates
    - sot:root/arks-cli-personality-factory
    - sot:root/arks-cli-output-exit-contract
    - sot:root/arks-lease
    - sot:root/storage-celir-boundary
    - sot:root/arks-daemon-product-intent
    - sot:root/arks-agent-product-intent
    - sot:root/arks-hub-product-intent
    - sot:root/verify-cleanroom-symmetry-matrix
    - sot:root/arks-dod-behavioral
    - sot:root/arks-dod-optimized
    - sot:root/verify-directory-conventions
    - sot:root/production-residual-register
    - research:root/20261113143029
    - research:root/20261113194123
  evidence: []
  governs:
    - verify/docker-compose.yml
---

# ARKS-native e2e coverage checklist

Document revision **0.1.2** (SoT edit counter only — **not** a product/crate release). Short inventory of **arks-only / product-native** surfaces that need Docker e2e confirmation beyond the three peer-symmetry pillars (git / git-daemon / gh). **No invent Claimed.** OUT/Refuse/later Deferred stay closed. Three-pillar Missing=0 is **not** reopened here. Behavioral must-pass: [[sot:root/arks-dod-behavioral]]. Floor: [[sot:root/arks-dod-optimized]].

## Rule

### Scope

1. Inventory is from owning product SoTs + `arks --help` / crate cmds — **do not invent** features.
2. Judge = Linux Docker `arks-verify-m1:0.1.0` with `ARKS_DOD_STRICT=1` where the slot defines STRICT.
3. Partial / non-goal SoT rows → **SKIP/Partial** in reports; do not invent harness Claimed.

### Checklist (service → surface)

| Native surface | Owning SoT | Compose / env | Claimed honesty |
|----------------|------------|---------------|-----------------|
| `arks serve` umbrella (HTTP+SSH+forge; `--no-*`) | [[sot:root/arks-self-host-server]] | `dod-self-host` | Hosting forge journeys Claimed (IW-FORGE-SERVE); cohost discoverability |
| Self-host hosting e2e | [[sot:root/arks-self-host-server]] | `dod-self-host` | Same |
| SSH B1/B2 (serve-related) | [[sot:root/arks-claimed-residual-gates]] | `dod-claimed-ssh` | Claimed |
| Hub HTTPS forge remote B3 | [[sot:root/arks-claimed-residual-gates]] | `dod-claimed-hub-https` | Claimed |
| Agent LLM / review B4 | [[sot:root/arks-claimed-residual-gates]] | `dod-claimed-agent-llm` | Claimed |
| Daemon upload-archive B5 | [[sot:root/arks-claimed-residual-gates]] | `dod-claimed-daemon-archive` | Claimed (`cmp -s`) |
| Daemon syslog B5+ | [[sot:root/arks-honesty-leftovers-wave]] | `dod-daemon-matrix` + `ARKS_DAEMON_B5_PLUS=1` | Claimed (collector PRI) |
| Rebind B7 HTTP/SSH/forge | [[sot:root/arks-claimed-residual-gates]] | `dod-claimed-rebind` | Claimed |
| In-process TLS B9 | [[sot:root/arks-claimed-residual-gates]] | `dod-claimed-tls` | Claimed |
| Residual umbrella | [[sot:root/arks-claimed-residual-gates]] | `dod-claimed-residual` | Orchestrates above |
| Personality drop-in / exit / survey | [[sot:root/arks-cli-personality-factory]] | `dod-cli-matrix` + `ARKS_PERSONALITY_*` | Sample+expand+exit+survey Claimed; peer drop-in |
| `arks` personality **unified** overlap I/O (bounded) | [[sot:root/arks-dod-optimized]] OD-UNIFIED | `dod-cli-matrix` + `ARKS_DOD_UNIFIED=1` | Floor OD — bounded smoke; full forge unified still Partial |
| `arks serve config` smoke | [[sot:root/arks-dod-optimized]] OD-SERVE-CFG | `dod-self-host` + `ARKS_DOD_SERVE_CFG=1` | Floor OD — path/show/set/validate; schema depth Partial |
| `arks commit` editor/refuse honesty | [[sot:root/arks-dod-optimized]] OD-COMMIT-HONESTY | `dod-cli-matrix` + `ARKS_DOD_COMMIT_HONESTY=1` | Floor OD — O3/FS1; interactive TUI non-DoD |
| BP-PORCELAIN everyday peer actions | [[sot:root/arks-dod-behavioral]] | `dod-cli-matrix` + `ARKS_DOD_BEHAVIORAL=1` | Behavioral DoD — named porcelain set; stubs ≠ PASS |
| Hub native auth/config (not gh clone) | [[sot:root/arks-hub-product-intent]] | `dod-hub-matrix` + `ARKS_HUB_IW_MUTATE=1` / UX | Claimed subsets |
| Lease product (arks-only) | [[sot:root/arks-lease]] | `g011-lease-product` (+ `g011-high-concurrency` L3) | Claimed M10/M23 |
| CELIR index dual-path (opt-in) | [[sot:root/storage-celir-boundary]], [[sot:root/celir-index-dual-path]] | `g022`–`g025` | Claimed M24; **CELIR admin CLI = non-product SKIP** |
| Serve smart HTTP / Basic (native host wire) | [[sot:root/arks-server]] | `g015-smart-http`, `g036-http-basic-auth` | Claimed G15/G36 |

### Explicit SKIP / OUT

| Item | Disposition |
|------|-------------|
| Three-pillar Missing=0 re-audit | Already audited — not this wave |
| Stub-Claimed stampede / OUT / Refuse / later Deferred | Must not reopen as must-pass |
| CLI Partial outside BP-PORCELAIN / SysV / L9 / G8 / later×119 | Non-DoD under [[sot:root/arks-dod-behavioral]]; hub terminal×6 must Claimed-or-Refuse |
| CELIR ObjectStore / CELIR admin UX | Refuse / cancelled non-product |
| WAN “secure by default” | Forbidden without ops evidence |
| Windows Event Log as B5+ | N/A |

### Verification

Operator wave: run STRICT Docker for each checklist row that has a harness; report PASS/FAIL/SKIP with reason. Spec-first amend this table when new native harnesses land.
