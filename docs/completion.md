---
title: Completeness
description: Completion milestone — design-complete CLI shell and honest Partial vs Claimed
nav_order: 66
schema: {faq: auto}
sections:
  - milestone
  - pillars
  - partial
  - non-goals
  - verify
  - {c: cta, name: next}
---

# Completeness

Short public answer to “is arks **done**?” — without inventing Claimed or PASS.

## Completion milestone (what “done” means here)

This site’s **completion milestone** is **design-complete + CLI-shell coherent**, not “every surface Claimed.”

| Layer | Milestone meaning |
|-------|-------------------|
| Design SoTs | Product intent, self-host (`arks serve`), personality factory, hub/daemon/agent intents, symmetry matrix, port policy — **active and cross-linked** |
| CLI shell | Single binary `arks`; personalities `git` \| `gh` \| `arks`; serve umbrella; flat forge leaves; daemon + agent groups **dispatchable** |
| Honesty | **Partial** / **Refuse** / **Missing** / fail-closed surfaces are **named in public docs** — stubs are not marketed as Claimed |
| Claimed bodies | Only where clean-room receipts exist (porcelain inventory, HTTP git wire, residual B-gates below, leases, timing bars, …) |

**One-word operator summary:** **PARTIAL** — symmetry design and shell are in place; residual Claimed gates **B1/B2/B3/B4/B5/B7(HTTP+SSH+forge)/B9** carry receipts; **B5+** **Refuse (terminal)**; IW-FORGE-SERVE hosting e2e Claimed; hub flat+expand Claimed (other v1 Partial; Missing=0); personality sample+expand Claimed (per-cmd tables Partial); live G8 / **L9** Not Claimed.

> [!IMPORTANT]
> Do **not** read this page as “all Claimed.” Design intent ≠ Claimed. Smoke/SKIP ≠ PASS. Residual gate SoT: [arks-claimed-residual-gates](https://github.com/modootoday/arks/blob/main/.spec/sot/arks-claimed-residual-gates.sot.md).

## Three symmetry DoD pillars

Public clean-room Definition of Done is **three peer pillars**. Self-host is **adjacent**, not a fourth Claimed pillar.

| Pillar | Stock peer | Product entry | Tip honesty (public) |
|--------|------------|---------------|----------------------|
| **1. CLI** | `git` | `arks <cmd>` / personality `git` | Inventory Claimed where matrices + receipts say so; unfinished flags fail closed |
| **2. Daemon** | `git-daemon` | `arks daemon` | Flag surface **Partial** deepen; archive byte-identical **B5 Claimed**; syslog facility **B5+ Refuse** |
| **3. Forge client** | `gh` | flat `arks pr` / `issue` / `auth` / … | Flat+expand **Claimed** where evidenced; other v1 **Partial** (Missing=0); **B3** HTTPS remote **Claimed**; nested `arks hub …` transitional |

**Related hosting:** `arks serve` (HTTP + SSH + forge API umbrella). HTTP git wire Claimed where G15 / G29–G32 / G36 evidence; in-process SSH **B1/B2 Claimed**; product in-process TLS **B9 Claimed**; HTTP+SSH+forge zero-downtime rebind **B7 Claimed**; forge-in-serve hosting e2e **Claimed** ([[evidence:root/20261111011425-self-host-forge-e2e]]). See [Self-host](../self-host/) and [Ports](../ports/).

Authoritative matrix: [verify-cleanroom-symmetry-matrix](https://github.com/modootoday/arks/blob/main/.spec/sot/verify-cleanroom-symmetry-matrix.sot.md). Residual ledger: [production residual register](https://github.com/modootoday/arks/blob/main/.spec/sot/production-residual-register.sot.md). Claimed residual gates: [arks-claimed-residual-gates](https://github.com/modootoday/arks/blob/main/.spec/sot/arks-claimed-residual-gates.sot.md). Program ladder: [program roadmap](https://github.com/modootoday/arks/blob/main/.spec/sot/program-roadmap.sot.md).

## Tip honesty (Claimed / Partial / Refuse bounds)

| Surface | Class | Notes |
|---------|-------|-------|
| Personality / Registry / OverlapFacade | **Claimed sample+expand** + Partial tables | Drop-in Docker sample+expand Claimed ([[evidence:root/20261111020515-personality-expand]]); per-cmd peer numeric tables Partial |
| `arks serve --no-ssh --no-forge` | HTTP wire **Claimed** where evidenced | Narrow Claimed hosting path |
| In-process SSH under `arks serve` | **B1/B2 Claimed** | OpenSSH-exact ChaCha wire + stock `git clone ssh://` e2e; durable §17(c) listen remains the product path; ops SSHD + pack still valid |
| Product in-process TLS | **B9 Claimed** | rustls PEM terminate + stock curl; ops reverse-proxy remains valid |
| Forge API under `arks serve` | **IW-FORGE-SERVE Claimed** | Hosting e2e PR/issue journeys ([[evidence:root/20261111011425-self-host-forge-e2e]]); **B3** is client↔HTTPS remote (separate) |
| `arks serve config` rebind | **B7 Claimed (HTTP+SSH+forge)** | Held-connection + dual-bind overlap; Windows file-mtime reload **Partial** (SIGHUP N/A) |
| `arks serve ssh-smoke` | **Partial** | Banner + `--git-pack` durable self smoke Partial — smoke alone ≠ **B1/B2** Claimed |
| Flat forge leaves (`pr`, `issue`, …) | **Claimed subset+expand** + Partial | Flat+expand Claimed ([[evidence:root/20261111011539-hub-flat-subset]], [[evidence:root/20261111020350-hub-full-expand]]); other v1 Partial (Missing=0); **B3** Claimed; OUT refuse |
| `arks daemon` deepen | **B5 Claimed** / **B5+ Refuse** | Archive byte-identical Claimed; syslog facility/priority **Refuse (terminal)**; classic double-fork **Refuse**; Windows honest Missing refuse |
| Remaining incompletes backlog | **Closed / terminal** | [[sot:root/arks-remaining-incompletes-wave]] — hub Missing→0 + expand Claimed; personality expand Claimed; B5+ Refuse |
| `arks agent` | **B4 Claimed** + smoke **Partial** | Docker fixture LLM chatbot/review e2e Claimed ([[evidence:root/20261110231959-claimed-agent-llm]]); `provider smoke` / `review --smoke` remain Partial |
| Live G8 latency / **L9** publish timing | **Not Claimed** | Secrets-absent SKIP ≠ PASS; L9 owner policy |

Git feature-symmetry DoD (**FS_final**) for claimed porcelain rows remains **YES** where receipts carry — residuals are Impossible / advisory / owner-policy, not casual N/A. Residual **B** rows above are Claimed / Partial / Refuse **only** as Spec + receipts name — do not invent beyond [[sot:root/arks-claimed-residual-gates]].

## Documented non-goals / trimmed

- Absolute SHA-1 collision **immunity** (detect+refuse is Claimed)
- Forge Checks / merge-queue / Issues **web product** UX (API client is flat on `arks`; UI not required for v1)
- Embedded forge OAuth browser UX (external credential.helper honored)
- Cloud-sync / hostfs product CLI (`arks host-env` trimmed)
- In-process LFS client/server (use external `git-lfs`)
- CELIR index as product Claimed path (ops/CI opt-in only; default OFF)
- Live G8 / scale latency PASS without secrets (honest SKIP)
- crates.io / docs.rs publish timing (**L9** — owner policy)
- Treating `arks hub serve` as the **primary** host story (superseded by `arks serve`)
- Spawning `git` or `gh` on product paths (forbidden — peers are clean-room oracles only)
- **B5+** syslog facility/priority Claimed (terminal **Refuse** this wave)

## Tip remasure notes (PF5)

G19 M17 ≤0.8× stays **19/19**. Aggressive ≤0.5×: hardlink + no PATH rewrite → **exactly one** sole Docker G19 → **claimed=19** / `aggressive_na=0` (ALIAS **0.4926**, REVERT **0.4227**) ([[evidence:root/20261111032000]]); prior G7 `aggressive_na` **0.5489** archival ([[evidence:root/20261111025200]]); **no** stampede; halt OOM. Historical **Claimed 8/8** at `79fc9bf` remains archival. Details: [Performance](../performance/).

## How maintainers verify

Clean-room Docker only — L8 (`g014-cli-parity-production`→`g016-journey-compare`→`g020-cli-symmetric-parity`→`g021-cli-flag-symmetry`) and S6 (`g038-gpgsig-openpgp`→`g039-partial-clone-filter`→`g040-dumb-http-push-deletes`) plus the compose matrix. Symmetry smoke: `dod-symmetry` (git / daemon / hub pillars). Residual Claimed slots: `dod-claimed-residual` / `dod-claimed-ssh` / `dod-claimed-hub-https` / `dod-claimed-agent-llm` / `dod-claimed-rebind` / `dod-claimed-tls` / `dod-claimed-daemon-archive`. Host CI is advisory. Pre-release CLI symmetry inventory is the `release-symmetry` gate before tagged `dist`. Receipts live under `.spec/evidence/`. Never invent PASS for secrets-absent live gates.

## Next

[Git vs arks](../git-vs-arks/) [Self-host](../self-host/) [CLI personality](../cli-personality/) [Limits](../limits/) [Performance](../performance/) [Contribute](../contribute/)
