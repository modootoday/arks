---
title: Ports
description: Normative Git transport ports vs demo and alternate binds
nav_order: 43
sections:
  - table
  - policy
  - honesty
  - {c: cta, name: next}
---

# Ports

Operator-facing clone URLs and docs prefer **IANA / Git-documented** ports. Unprivileged loopback demos may bind higher ports — those are **alternates**, never “more standard” than the normative set.

## Normative vs demo

| Surface | Normative target | Demo / alternate | Notes |
|---------|------------------|------------------|-------|
| SSH git (`arks serve` SSH module) | **22** | **2222** | 22 = IANA `ssh`; 2222 = unprivileged / host-`sshd` conflict workaround (IANA EtherNet/IP — **not** a Git port) |
| HTTP git | **80** | **8080** | 80 = IANA `http`; 8080 = unprivileged demo |
| HTTPS | **443** | Ops TLS front on 80/8080 | Prefer reverse-proxy TLS off localhost |
| `git://` (`arks daemon`) | **9418** | — | IANA `git` + git-daemon default |
| Forge API (under `arks serve`) | **3920** | configurable | arks-local convention — not an IANA Git port |

## Policy

1. Teach **22 / 80 / 443 / 9418** as the product story when the operator can arrange privileged bind, capability, or an ops front (reverse proxy / NAT / publish map).
2. Label **2222 / 8080** as **demo / alternate** in help and docs.
3. Process clap defaults may stay loopback-friendly demos until a coordinated code + test + docs flip — defaults must **not** be described as more normative than 22/80.
4. Port policy ≠ Claimed for in-process SSH handshake, in-process TLS, or WAN-safe hosting. SSH demo bind (**2222**) may be **Partial**; binding a port is not handshake Claimed.

Full self-host bind/auth story: [Self-host](../self-host/). Research: [ports survey](https://github.com/modootoday/arks/blob/main/.spec/research/20261111055609-git-transport-standard-ports.research.md). SoT: [arks-self-host-server](https://github.com/modootoday/arks/blob/main/.spec/sot/arks-self-host-server.sot.md).

## Honesty

Binding a port does not invent Claimed module readiness. In-process SSH may prove bind + key paths (**Partial**) and still **fail closed** before handshake. If SSH or forge modules are enabled and not ready past their Partial wire, `arks serve` **fails closed**. **HTTPS** / in-process TLS remote remains **Missing**.

## Next

[Self-host](../self-host/) [Daemon](../daemon/) [Completeness](../completion/) [Limits](../limits/)
