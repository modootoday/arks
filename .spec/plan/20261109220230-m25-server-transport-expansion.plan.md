---
id: plan:root/20261109220230
title: M25 server transport expansion
status: active
created: 2026-11-09T22:02:30Z
updated: 2026-11-09T22:02:30Z
tags: [milestone, m25, phase-sv2, server, ssh, git-daemon, dumb-http, protocol-v2]
rel:
  depends_on:
    - sot:root/program-roadmap
    - sot:root/arks-server
    - sot:root/cli-surface
    - sot:root/interop-test-matrix
    - sot:root/product-verification-clean-room
    - sot:root/production-residual-register
    - sot:root/remote-host-compatibility-surface
    - sot:root/git-compat-surface
    - plan:root/20261108105446
    - adr:root/20261109220230
    - research:root/20261108071916
    - evidence:root/20261108231300
  implements: [sot:root/program-roadmap, adr:root/20261109220230]
---

# M25 — Server transport expansion (Phase Sv2)

## Goal

Claim four previously refuse-closed server surfaces against stock Git, with primary-doc fidelity and Docker clean-room receipts — without rewriting **M11** smart HTTP (**G15**).

Unlock authority: [[adr:root/20261109220230]].

## Claim set

| ID | Surface | Primary docs | Gate | CLI / crate surface |
|----|---------|--------------|------|---------------------|
| FW-SRV-DUMB | Dumb HTTP (additive; smart HTTP remains required) | [gitprotocol-http](https://git-scm.com/docs/gitprotocol-http) dumb section; `update-server-info` | **G29** | `serve-http` serves dumb GETs; `update-server-info` |
| FW-SRV-SSH | SSH command-mode (stdio pack protocol) | [gitprotocol-pack](https://git-scm.com/docs/gitprotocol-pack), [git-upload-pack](https://git-scm.com/docs/git-upload-pack), [git-receive-pack](https://git-scm.com/docs/git-receive-pack) | **G30** | duplex `arks upload-pack` / `arks receive-pack` |
| FW-SRV-V2 | Protocol v2 (`ls-refs` + `fetch` subset) | [gitprotocol-v2](https://git-scm.com/docs/gitprotocol-v2) | **G31** | smart HTTP + stdio when `version=2` requested |
| FW-SRV-GIT | `git://` daemon | [git-daemon](https://git-scm.com/docs/git-daemon), gitprotocol-pack | **G32** | `arks daemon` |

## Ordered work

1. **SoT amend** — arks-server claim matrix; residual §C/§F; roadmap M25; interop G29–G32; cli-surface Wave Sv; matrix files.
2. **Inner modules (bottom-up)** — pack-protocol duplex advertisement (no HTTP `# service=` prefix); dumb static paths + `update-server-info`; v2 capability / ls-refs / fetch; git-daemon request parse + serve loop.
3. **CLI compose** — fix upload-pack/receive-pack duplex; extend `serve-http`; add `daemon`; help honesty.
4. **Verify** — unit/host advisory; sole-runner Docker G29→G30→G31→G32; evidence per transport; `spec check`; commit+push each closed transport.

## Out of scope

- Product SSHD / key CA / forge OAuth.
- Server-side githooks (FW-SRV-HOOKS remains refuse-unimplemented).
- Dumb-HTTP-only as sole production-server path.
- Invented capabilities; `delete-refs` / `report-status-v2` unless separately claimed.
- CELIR ObjectStore; redefining M11 G15.

## Exit

Each transport is **Claimed** only with Docker PASS evidence for its gate. M25 Done = G29–G32 all Claimed + residual rows updated + public honesty.

## Cross-links

[[adr:root/20261109220230]], [[sot:root/arks-server]], [[sot:root/program-roadmap]], [[sot:root/interop-test-matrix]], [[plan:root/20261108105446]]
