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
| Honesty | **Partial** / **Missing** / fail-closed surfaces are **named in public docs** — stubs are not marketed as Claimed |
| Claimed bodies | Only where clean-room receipts exist (porcelain inventory, HTTP git wire, leases, timing bars, …) |

**One-word operator summary:** **PARTIAL** — symmetry design and shell are in place; deep forge/SSH/agent bodies and hosting e2e remain Partial or Missing.

> [!IMPORTANT]
> Do **not** read this page as “all Claimed.” Design intent ≠ Claimed. Smoke/SKIP ≠ PASS.

## Three symmetry DoD pillars

Public clean-room Definition of Done is **three peer pillars**. Self-host is **adjacent**, not a fourth Claimed pillar.

| Pillar | Stock peer | Product entry | Tip honesty (public) |
|--------|------------|---------------|----------------------|
| **1. CLI** | `git` | `arks <cmd>` / personality `git` | Inventory Claimed where matrices + receipts say so; unfinished flags fail closed |
| **2. Daemon** | `git-daemon` | `arks daemon` | Flag surface **Partial** deepen; not the GitHub-class host story |
| **3. Forge client** | `gh` | flat `arks pr` / `issue` / `auth` / … | **Partial** (stubs/forwards); nested `arks hub …` transitional only |

**Related hosting:** `arks serve` (HTTP + SSH + forge API umbrella). HTTP git wire Claimed where G15 / G29–G32 / G36 evidence; in-process SSH **Partial** (bind + keys + id/KEXINIT scaffold fail-closed; KEX crypto/auth still **Missing** — not handshake Claimed); forge-in-serve **Partial**; dedicated LAN/WAN hosting e2e **scaffold / not Claimed**. See [Self-host](../self-host/) and [Ports](../ports/).

Authoritative matrix: [verify-cleanroom-symmetry-matrix](https://github.com/modootoday/arks/blob/main/.spec/sot/verify-cleanroom-symmetry-matrix.sot.md). Residual ledger: [production residual register](https://github.com/modootoday/arks/blob/main/.spec/sot/production-residual-register.sot.md). Program ladder: [program roadmap](https://github.com/modootoday/arks/blob/main/.spec/sot/program-roadmap.sot.md).

## Tip Partial / Missing (honest bounds)

| Surface | Class | Notes |
|---------|-------|-------|
| Personality / Registry / OverlapFacade | **Partial** | Env/argv0 dispatch exists as shell; not Claimed peer drop-in proof |
| `arks serve --no-ssh --no-forge` | HTTP wire **Claimed** where evidenced | Practical Claimed hosting path today |
| In-process SSH under `arks serve` | **Partial** | Bind + keys + id + KEXINIT + curve25519 ECDH Partial; fail-closed (host-key **sign**/auth/channel **Missing**). Ops SSHD + `upload-pack`/`receive-pack` remains valid |
| Forge API under `arks serve` | **Partial** | Local API scaffold ≠ Claimed hosting e2e; no in-process TLS Claimed |
| `arks serve config` | **Partial** | File seeds + `runtime_*` / `runtime_probe_*` + `apply`/`reload` probes; clap wins; ≠ Claimed sync e2e |
| Flat forge leaves (`pr`, `issue`, …) | **Partial** | Many local-API Partial leaves; HTTPS remote via ureq Partial; OUT leftovers `extension`/`copilot` refuse |
| `arks daemon` deepen | **Partial** | `--inetd` / `--pid-file` / `--detach` (setsid + parent pid-file) / archive ACK+sideband band1+band2 Partial; still **Missing**: Claimed archive parity, hooks/user/group |
| `arks agent` | **Partial** | provider/config/doctor + review/cev/run + serve/bot health + chat proxy (JSON + stream SSE Partial); Claimed LLM chatbot e2e **Missing** |
| Live G8 latency / **L9** publish timing | **Not Claimed** | Secrets-absent SKIP ≠ PASS; L9 owner policy |

Git feature-symmetry DoD (**FS_final**) for claimed porcelain rows remains **YES** where receipts carry — residuals are Impossible / advisory / owner-policy, not casual N/A. That does **not** promote forge/SSH/agent Partial rows to Claimed.

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

## Tip remasure notes (PF5)

G19 M17 ≤0.8× stays **19/19**. Aggressive ≤0.5×: hardlink + no PATH rewrite → **exactly one** sole Docker G19 → **claimed=19** / `aggressive_na=0` (ALIAS **0.4926**, REVERT **0.4227**) ([[evidence:root/20261111032000]]); prior G7 `aggressive_na` **0.5489** archival ([[evidence:root/20261111025200]]); **no** stampede; halt OOM. Historical **Claimed 8/8** at `79fc9bf` remains archival. Details: [Performance](../performance/).

## How maintainers verify

Clean-room Docker only — L8 (`g14`→`g16`→`g20`→`g21`) and S6 (`g38`→`g39`→`g40`) plus the compose matrix. Symmetry smoke: `dod-symmetry` (git / daemon / hub pillars). Host CI is advisory. Pre-release CLI symmetry inventory is the `release-symmetry` gate before tagged `dist`. Receipts live under `.spec/evidence/`. Never invent PASS for secrets-absent live gates.

## Next

[Git vs arks](../git-vs-arks/) [Self-host](../self-host/) [CLI personality](../cli-personality/) [Limits](../limits/) [Performance](../performance/) [Contribute](../contribute/)
