---
id: adr:root/20261110214800
title: Claim LFS network fetch/server + sha256 porcelain CLI
status: accepted
created: 2026-11-10T21:48:00Z
updated: 2026-11-10T21:48:00Z
tags: [lfs, sha256, full-symmetry, porcelain]
rel:
  depends_on:
    - research:root/20261110212414
    - adr:root/20261110214000
    - sot:root/arks-remote
    - sot:root/arks-server
    - sot:root/production-residual-register
---

# Claim LFS network fetch/server + sha256 porcelain CLI

## Context

Owner DoD still requires **complete git symmetry**. Prior FS reopen closed local LFS smudge and sha256 *refuse*. Remaining implementable gaps: Git LFS **batch/basic network** path (client + server) and **`extensions.objectFormat=sha256` porcelain** end-to-end where Git supports it.

## Decision

| Surface | Claimed slice | Explicit non-claim / Impossible |
|---------|---------------|----------------------------------|
| **LFS network client** | On checkout smudge, if pointer lacks local object: discover LFS endpoint (`lfs.url` / remote `…/info/lfs` / `ARKS_LFS_URL`); POST batch `download`; GET basic transfer; store under `$GIT_DIR/lfs/objects`; size-check | tus/custom transfers; LFS locks API product; full `git-lfs` CLI clone |
| **LFS server** | `arks serve-http` answers `POST …/info/lfs/objects/batch` + basic GET/PUT `…/info/lfs/objects/{oid}` against `$GIT_DIR/lfs/objects` | Hosted multi-tenant LFS; S3 redirect product |
| **`arks lfs`** | In-process `fetch`/`pull` (batch download missing pointers) + `push` (batch upload); other argv still hand off to `git-lfs` when present | Pixel `git-lfs` UX identity |
| **sha256 porcelain** | `arks init --object-format=sha256`; open/read/write index+objects with Sha256; `hash-object`/`add`/`commit`/`checkout`/`fsck`/`status`/`log` on sha256 repos; remotes/server use repo object format for pack OID width | Cryptographic SHA-1 collision immunity (**Impossible**); mixed sha1↔sha256 remotes |

## Consequences

Flip residual **FW-LFS** and sha256 porcelain non-claim to **Claimed**. Keep Impossible: SHA-1 collision immunity, forge Checks/PR UX, L9, live G8, host/env FS-007/FS-014/WIN-003 (mitigation notes only). No invent PASS.
