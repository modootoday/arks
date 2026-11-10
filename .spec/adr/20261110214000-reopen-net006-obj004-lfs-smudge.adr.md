---
id: adr:root/20261110214000
title: Reopen NET-006 / OBJ-004 / in-process LFS smudge (close FS harness residuals)
status: accepted
created: 2026-11-10T21:40:00Z
updated: 2026-11-10T21:40:00Z
tags: [net-006, obj-004, lfs, smudge, full-symmetry, reopen]
rel:
  depends_on:
    - research:root/20261110212414
    - adr:root/20261110213500
    - sot:root/arks-remote
    - sot:root/multi-agent-git-concurrency-scenarios
    - sot:root/production-residual-register
---

# Reopen NET-006 / OBJ-004 / in-process LFS smudge

## Context

FS0–FS_final marked Done with three **explicit reopen harnesses** (not casual N/A): HTTP bundle-uri races (NET-006), SHA-1/sha256 hostile content (OBJ-004), and in-process LFS smudge beyond `filter.*.clean` / external `git-lfs` handoff. Owner DoD still requires complete git symmetry — close these with Claimed slices or Impossible+closest.

## Decision

| Row | Claimed slice | Explicit Impossible / non-claim |
|-----|---------------|----------------------------------|
| **NET-006** | `clone --bundle-uri` accepts `http(s)://` URLs; atomic temp download; **Content-Length** mismatch / empty body refuse (no unbundle of partial); concurrent download under shared `*.lock` refuse (serialize) | Guaranteeing remote CDN race-freedom for all third-party bundle hosts; server `bundle-uri` capability advertise (separate SoT) |
| **OBJ-004** | `arks fsck` recomputes content OID for every reachable object and **refuse on OidMismatch**; `extensions.objectFormat=sha256` CLI open **refuse** with reason (library sha256 remains Claimed in hash/store/pack) | Cryptographic immunity to theoretical SHA-1 collisions (Impossible as a feature); full sha256 porcelain product |
| **LFS smudge** | Checkout materialize honors `filter.<name>.smudge`; **in-process** Git LFS pointer → `.git/lfs/objects/…` when present (size check) | Full LFS transfer/server product; network fetch of missing LFS objects (still `git-lfs` / filter helper) |

## Consequences

Flip residual **FW-CAT-NET-006**, **FW-CAT-OBJ-004**, **FW-LFS** (smudge) to Claimed/closest. Update research FS5/FS6/FS8/FS_final rows, `docs/git-vs-arks.md`, catalog dispositions. **L9** + live G8 remain non-FS. No invent PASS.
