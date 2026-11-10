---
id: research:root/20261111074523
title: Claimed residual gates — deps, FFI/TLS/SSH, Docker verify shape
status: active
created: 2026-11-10T22:45:23Z
updated: 2026-11-10T22:45:23Z
tags: [claimed, residual, docker, clean-room, ssh, openssh, tls, hub, agent, daemon, archive, syslog, rebind, no-invent-claimed, ffi]
verify: manual
rel:
  depends_on:
    - research:root/20261111072342
    - sot:root/arks-self-host-server
    - sot:root/arks-server
    - sot:root/arks-hub-product-intent
    - sot:root/arks-agent-product-intent
    - sot:root/arks-daemon-product-intent
    - sot:root/arks-cli-personality-factory
    - sot:root/verify-cleanroom-symmetry-matrix
    - sot:root/product-verification-clean-room
    - sot:root/production-residual-register
    - sot:root/language-english-only
  related:
    - verify/images.lock.json
    - verify/docker-compose.yml
---

# Claimed residual gates — deps, FFI/TLS/SSH, Docker verify shape

Tip surveyed: **`4f87ad8`** (canon `C:\workspaces\gir`). English-only. **Design ≠ Claimed.** No invent Claimed. Product paths never spawn `git` / `gh` / `ssh` / `ollama`.

## Purpose

Promote [[research:root/20261111072342]] blocked rows **B1 / B2 / B3 / B4 / B5 / B7 / B9** (+ Claimed **syslog facility** if graduated) into a **clean-room Docker verify shape** so later implement waves have acceptance slots without inventing PASS from Partial smokes.

**Out of this research:** B6 durable SSH listen/rebind Partial (already landed); B8 Windows SIGHUP (**N/A**).

## Cross-OS honesty

| Venue | Role |
|-------|------|
| Windows host PowerShell | Drive `docker compose` only — **not** normative DoD |
| Linux Docker `arks-verify-m1:0.1.0` | **Authoritative** bash + pin stock Git **2.39.5** + OpenSSH packages per `verify/images.lock.json` |

## Deps / libraries (cite tip crates)

| Area | Workspace / crate deps | Honesty |
|------|------------------------|---------|
| SSH crypto Partial | `arks-server`: `chacha20poly1305`, `x25519-dalek`, `ed25519-dalek` | Partial AEAD/KEX ≠ OpenSSH-exact packet interop (**B1**) |
| SSH stack policy | **No** `russh` / `libssh` / OpenSSH server embed in product | Keep pure Rust + labeled Partial; Claimed needs live OpenSSH **client** evidence |
| Hub HTTPS Partial | `arks-hub`: `ureq` + smoke `rustls` / `rustls-pemfile` | `--local-tls` smoke ≠ forge remote Claimed (**B3**); product in-process TLS terminate still **B9** |
| Agent LLM Partial | Native TCP/HTTP openai-compat (`arks-agent`); never spawn `ollama` CLI | Mock/smoke ≠ multi-turn Claimed e2e (**B4**) |
| Daemon syslog / user / group Partial | `arks` CLI unix `libc` (`openlog`/`syslog`, `setuid`/`setgid`) | Facility/priority parity vs stock git-daemon = Claimed residual; Windows Event Log ≠ syslog |
| FFI boundary | `arks-server` `forbid(unsafe_code)` | Privilege/syslog stay in CLI; server crate stays safe Rust |

## OpenSSH image fact

Clean-room image already records OpenSSH client/server packages and expected version string in `verify/images.lock.json` (`OpenSSH_9.2p1` bookworm). **B1/B2** gates MAY use stock `ssh` / `ssh-keygen` **inside the container as oracles**, never from the product binary.

## Proposed Docker verify slots (scaffold / SKIP until Claimed)

| ID | Gate | Compose service | Entrypoint | Claimed bar (summary) |
|----|------|-----------------|------------|------------------------|
| B1 | OpenSSH-exact cipher/wire | `dod-claimed-ssh` | `verify/scripts/run-claimed-ssh-e2e.sh` | Live OpenSSH client completes AEAD packet path vs in-process `arks serve` SSH; receipt proves cipher selection |
| B2 | Claimed SSH git e2e | same service (depends B1) | same script | Stock `git` clone/fetch/push over SSH to durable `arks serve` SSH; clean-room receipt |
| B3 | Hub HTTPS forge remote e2e | `dod-claimed-hub-https` | `verify/scripts/run-claimed-hub-https-e2e.sh` | `arks hub` / flat forge leaves against real HTTPS forge base (local `arks serve` forge or fixtures); no `gh` spawn |
| B4 | Agent LLM chatbot/review e2e | `dod-claimed-agent-llm` | `verify/scripts/run-claimed-agent-llm-e2e.sh` | Multi-turn chat + review product path vs openai-compat fixture daemon; never invent PASS from `--smoke` |
| B5 | Daemon archive parity | `dod-claimed-daemon-archive` | `verify/scripts/run-claimed-daemon-archive-e2e.sh` | Byte/stream compare `upload-archive` vs stock `git-daemon` pin |
| B5+ | Syslog facility Claimed | same service (optional lane) | same script | Facility/priority parity vs stock when `--syslog` Claimed; Partial openlog ≠ Claimed |
| B7 | Zero-downtime rebind | `dod-claimed-rebind` | `verify/scripts/run-claimed-rebind-e2e.sh` | Connection-drain / dual-bind evidence under serve-config reload; Partial live rebind ≠ Claimed |
| B9 | Product in-process TLS | `dod-claimed-tls` | `verify/scripts/run-claimed-tls-e2e.sh` | Product TLS terminate on `arks serve` (not ops reverse-proxy alone); smoke self-signed ≠ Claimed |
| ALL | Orchestrator | `dod-claimed-residual` | `verify/scripts/run-claimed-residual-dod.sh` | Runs all slots; default SKIP/scaffold; `ARKS_DOD_STRICT=1` fails closed until Claimed |

## Partial vs Claimed (hard law)

1. Tip Partial smokes (`ssh-smoke`, hub `--local-tls`, agent `--smoke`/`--mock`, daemon archive protocol-body tests, Partial live rebind) **MUST NOT** flip any row to Claimed.
2. Scaffold scripts exit **0** with explicit `SKIP/scaffold` unless `ARKS_DOD_STRICT=1` (then exit **1** until evidence + owning SoT Claimed amend).
3. Host `cargo test` / PowerShell are advisory only ([[sot:root/product-verification-clean-room]]).

## Relationship to existing SoTs

| SoT | Relationship |
|-----|--------------|
| [[sot:root/arks-self-host-server]] | Owns serve umbrella, ports, §17 Partial durable SSH; B1/B2/B7/B9 graduate under its honesty |
| [[sot:root/arks-cli-personality-factory]] | CLI dispatch; does not invent SSH/TLS Claimed |
| [[sot:root/verify-cleanroom-symmetry-matrix]] | Three symmetry pillars stay; claimed-residual slots are **adjacent** hosting/agent/daemon deepen — not a fourth symmetry pillar |
| [[sot:root/arks-hub-product-intent]] / agent / daemon intent | Own product surfaces for B3 / B4 / B5 |
| [[sot:root/production-residual-register]] | Post-terminal next-queue cites these gates as reopen-only Design until evidenced |

## Non-goals

- Invent Claimed/PASS from this research or from SKIP harness greens.
- Embed OpenSSH/libssh in product.
- Treat ops reverse-proxy TLS as product in-process TLS Claimed (B9).
- Push remotes.
- Hangul in Specfold.
---

Normative design SoT promoting these slots: [[sot:root/arks-claimed-residual-gates]] (landed with this research wave).
