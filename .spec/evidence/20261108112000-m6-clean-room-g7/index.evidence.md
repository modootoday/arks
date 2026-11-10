---
id: evidence:root/20261108112000
title: M6 clean-room G7 Wave H remote client
status: active
created: 2026-11-08T11:20:00Z
updated: 2026-11-08T11:20:00Z
tags: [m6, clean-room, g0, g1, g2, g3, g4, g5, g6, g7, g10, wave-h, remote, verified, v0.3.1]
rel:
  depends_on:
    - sot:root/product-verification-clean-room
    - sot:root/interop-test-matrix
    - sot:root/program-roadmap
    - plan:root/20261108090651
    - sot:root/arks-remote
    - sot:root/arks
    - sot:root/cli-surface
    - sot:root/remote-host-compatibility-surface
    - adr:root/20261108083507
    - evidence:root/20261108104837
---

# M6 clean-room G7 Wave H receipts (roadmap v0.3.1+)

## Context

Milestone **M6** (Remote client / Phase H) under program roadmap **v0.3.1+**. Authoritative venue: Docker clean-room under `verify/` per [[sot:root/product-verification-clean-room]]. Image pins: `verify/images.lock.json`. Matrix: `verify/matrix-m6.md`. Unit SoTs: [[sot:root/arks-remote]], [[sot:root/arks]]. Depends on M5 [[evidence:root/20261108104837]].

Generic smart-HTTP **client** for any claimed peer; GitHub-class hosts are the verify-peer label only ([[sot:root/remote-host-compatibility-surface]]). Host `cargo test` results are **advisory only**.

## Environment

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` |
| Image | `arks-verify-m1:0.1.0` |
| Image digest (local Id) | `sha256:eb644ff80a051b6bd29ec76b773620b4387e40f07e33c97e54c3f2591157879f` |
| Base | `rust:1.99.1-bookworm@sha256:e6f2cff35d0f309acf8433b4f05878ed77092602c5065cc81801177da0bb3a9c` |
| Stock Git (in-image) | `git version 2.39.5` |
| rustc (in-image) | `rustc 1.99.1 (58cf8f9c3 2026-10-13)` |
| `CARGO_BUILD_JOBS` | `2` |
| Host Docker engine | Linux (`desktop-linux`) |
| Tree SHA (implementation commit) | `07f2286c80761b8e0cd4fd4e15c33de8dcab2615` |
| Tree SHA (post-exit hardening; G7 re-verified) | `4ab94e2ecced9d6193e7e8897163b6186a510d5e` |
| Workspace members exercised | Rings 0–6 libs + `arks` Wave L+H bin |

## Results

| Gate | Command | Outcome | Notes |
|------|---------|---------|-------|
| **gix-exit** | `cargo metadata` graph scan in `run-g7.sh` | **PASS** | zero `gix` / `gitoxide` edges |
| **G7** | `docker compose -f verify/docker-compose.yml run --rm g7` | **PASS** | `verify/g7` smart-HTTP suite **3** (help/refuse, remote config, clone/fetch/push loopback) |
| **G6** regression | (embedded in `run-g7.sh`) | **PASS** | `verify/g6` **6** |
| **G3** regression | (embedded) | **PASS** | `verify/g3` **3** |
| **G4** regression | (embedded) | **PASS** | `verify/g4` **3** |
| **G5** regression | (embedded) | **PASS** | `verify/g5` **4** + `arks-protocol` |
| **G2** regression | (embedded) | **PASS** | `verify/g2` **9** |
| **G10** regression | (embedded) | **PASS** | `verify/g10` **7** |
| **G1** regression | (embedded) | **PASS** | `arks-object` `g1_oracle` **5** + `verify/g1` **6** |
| Unit Specfold | `spec check` | **PASS** | 0 errors / 0 warnings |

Combinatorics covered (automated): M6-C01…C15 per `verify/matrix-m6.md`. Residual Open: M6-C16 G8 live smoke; M6-C17 G9 filters; M6-C18 SSH.

## Bug simulation (fixed)

1. `verify/scripts/run-g7.sh` initially had CRLF endings → `set: pipefail` failed in Linux clean-room; rewritten to LF (`.gitattributes` `*.sh text eol=lf`).
2. G6 help test still forbade Wave H verbs after Wave H landed; updated to require `remote`/`fetch`/`push`/`clone` and refuse unclaimed `pull`.

## Non-claims

- **Not production** — production is **M9** CLI-parity closure.
- No G8 live forge smoke (network/secrets policy **Open**).
- No SSH remotes; no `pull` (needs Pa merge/rebase).
- No Git server / `arks-server` (**M11**).
- No forge APIs (B); no CELIR; no L1+ lease runtime (**M10**).
- Workspace Cargo graph: **zero** `gix`/gitoxide edges.

## Reproduce

```bash
docker compose -f verify/docker-compose.yml build toolchain
docker compose -f verify/docker-compose.yml run --rm g7
```
