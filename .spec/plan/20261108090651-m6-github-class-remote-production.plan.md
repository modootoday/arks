---
id: plan:root/20261108090651
title: M6 GitHub-class remote client
status: active
created: 2026-11-08T09:06:51Z
updated: 2026-11-08T11:20:00Z
tags: [milestone, m6, phase-h, remotes, client]

rel:
  depends_on:
    - sot:root/program-roadmap
    - plan:root/20261108090650
    - sot:root/remote-host-compatibility-surface
    - sot:root/cli-surface
    - sot:root/public-surface
    - sot:root/interop-test-matrix
    - sot:root/product-verification-clean-room
    - adr:root/20261108083507
  implements: [sot:root/program-roadmap]
---

# M6 — Remote client (Phase H)

## Goal

Ship a **generic** Git remote **client** for claimed smart HTTP/SSH (etc.) protocols (surface A) and Wave **H** CLI (`remote`/`fetch`/`push`/`clone`, and `pull` only where claimed). **GitHub-class hosts are the representative clean-room / live-smoke verify peer** — not the exclusive product remote type ([[sot:root/remote-host-compatibility-surface]]). **Not production** — required before M9, but porcelain waves M7–M9 remain. Former v0.2 “M6 = production” is **revoked**.

## In scope / crates

| Artifact | Role |
|----------|------|
| `crates/arks-remote` | HTTPS (+ SSH when claimed) smart protocol **client** for any peer speaking claimed protocols |
| CLI Wave H | `remote`, `fetch`, `push`, `clone` on `arks` bin |
| Verify peer | GitHub-class recorded (**G7**) / live (**G8**) fixtures |
| Honesty | Public/help claim remotes **client** subset only |

## Out of scope

- arks as Git **server** — deferred to **M11** / [[plan:root/20261108105446]] (locked goal; not Open).
- Host product APIs (B).
- CELIR activation; lease runtime (M10).
- Full porcelain inventory / CLI-parity production (M9).
- Requiring G8 live smoke when network policy still Open (G7 recorded is CI default).

## Depends on

**M5** exit.

## Exit gates

| Gate | Clean-room | Advisory |
|------|------------|----------|
| **G7** remote-recorded | Authoritative CI default when live creds absent | Host |
| **G8** remote-live-smoke | Only when secrets/network policy closed | — |
| **G9** partial-shallow | Only if filters claimed | — |
| G1–G6 regression | Clean-room | Host |

## Ordered work items

1. **Design** — Claimed transport matrix (HTTPS required; SSH optional claim); recorded fixture redaction rules.
2. **Scaffold** — `arks-remote` + CLI Wave H wiring.
3. **Implement** — fetch/clone paths first, then push/report-status; auth via env/helpers only.
4. **Verify** — G7 fixtures; optional G8/G9; public-surface honesty (no production claim).

## Open questions

- G8 network policy default — clean-room SoT Open.
- SSH-over-443 claim timing.
- Incomplete `pull` refuse until Pa/Pb — [[sot:root/cli-surface]].

## Cross-links

[[sot:root/program-roadmap]], [[sot:root/remote-host-compatibility-surface]], [[sot:root/cli-surface]], [[sot:root/public-surface]], [[adr:root/20261108083507]]
