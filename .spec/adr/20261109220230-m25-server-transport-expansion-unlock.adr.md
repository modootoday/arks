---
id: adr:root/20261109220230
title: M25 server transport expansion unlock
status: active
created: 2026-11-09T22:02:30Z
updated: 2026-11-09T22:02:30Z
tags: [m25, server, transport, ssh, git-daemon, dumb-http, protocol-v2, refuse-unlock, phase-sv]
rel:
  depends_on:
    - sot:root/arks-server
    - sot:root/program-roadmap
    - sot:root/production-residual-register
    - sot:root/interop-test-matrix
    - sot:root/product-verification-clean-room
    - sot:root/cli-surface
    - sot:root/remote-host-compatibility-surface
    - plan:root/20261108105446
    - research:root/20261108071916
    - evidence:root/20261108231300
  evidence: []
---

# M25 server transport expansion unlock

## Context

**M11** / Phase **Sv** production-server exited on smart HTTP only ([[evidence:root/20261108231300]], [[sot:root/arks-server]] v0.1.0). Residual rows **FW-SRV-SSH**, **FW-SRV-GIT**, **FW-SRV-DUMB**, and **FW-SRV-V2** were **Refuse-closed (terminal)** under M22 so help/public copy would not pretend those surfaces shipped.

Primary docs already catalogue the wire shapes: [gitprotocol-pack](https://git-scm.com/docs/gitprotocol-pack) (SSH / `git://` / file duplex), [gitprotocol-http](https://git-scm.com/docs/gitprotocol-http) (dumb + smart), [gitprotocol-v2](https://git-scm.com/docs/gitprotocol-v2), [git-daemon](https://git-scm.com/docs/git-daemon), [git-upload-pack](https://git-scm.com/docs/git-upload-pack) / [git-receive-pack](https://git-scm.com/docs/git-receive-pack). Catalog: [[research:root/20261108071916]].

Operator intent now unlocks those four surfaces as **claimable** under a new milestone (**M25** / Phase **Sv2**) without rewriting the M11 smart-HTTP receipt.

## Decision

1. **Supersede refuse-closed** for **FW-SRV-SSH**, **FW-SRV-GIT**, **FW-SRV-DUMB**, and **FW-SRV-V2**. They become **claimable** under [[plan:root/20261109220230]] — not Claimed until Docker clean-room PASS + evidence for that transport.
2. **Milestone M25** (Phase **Sv2**) owns the expansion. **M11 / G15** remain the frozen smart-HTTP production-server bar; new gates **G29–G32** carry per-transport authority (see Consequences).
3. **Claim rules (normative):**
   - **SSH command-mode:** duplex gitprotocol-pack over stdio when the peer invokes `arks upload-pack` / `arks receive-pack` (same shape as stock `git-upload-pack` / `git-receive-pack` under SSH). No product SSHD / key auth server; operators may wrap ForceCommand / `git-shell`.
   - **`git://` daemon:** TCP listener per [git-daemon](https://git-scm.com/docs/git-daemon) request pkt-line + upload-pack (receive-pack only when explicitly enabled for the claim subset). Export marker / `--export-all` honesty required.
   - **Dumb HTTP:** additive path beside smart HTTP — never sole claimed server path. Serve `info/refs` (no service query), `objects/info/packs`, loose/pack GETs per [gitprotocol-http](https://git-scm.com/docs/gitprotocol-http); `update-server-info` must refresh advertisements.
   - **Protocol v2:** advertise and execute claimed commands only (`ls-refs`, `fetch` subset) per [gitprotocol-v2](https://git-scm.com/docs/gitprotocol-v2). Never invent caps; v1 remains available when clients do not request v2.
4. **Still refuse:** product forge auth, server-side githooks (until separately claimed), `delete-refs` / `report-status-v2` until advertised+implemented, CELIR ObjectStore, invented wire semantics.
5. **Verification:** host unit tests are advisory. Product Claimed requires sole-runner Docker clean-room for the owning gate. Serialize Docker; parallelize independent transport implementation safely.

## Consequences

- [[sot:root/arks-server]], [[sot:root/cli-surface]] Wave Sv, [[sot:root/production-residual-register]] §C/§F, [[sot:root/program-roadmap]], and [[sot:root/interop-test-matrix]] MUST amend to match this unlock.
- New clean-room gates (authoritative):
  - **G29** `server-dumb-http` — FW-SRV-DUMB
  - **G30** `server-ssh-command` — FW-SRV-SSH
  - **G31** `server-protocol-v2` — FW-SRV-V2
  - **G32** `server-git-daemon` — FW-SRV-GIT
- Residual disposition moves from Refuse-closed → **Unlocked (claimable)** until each gate PASS, then **Claimed** with evidence. Re-refuse requires a new ADR.
- Public copy MAY mention these transports only after the matching Claimed evidence exists.
- Does **not** reopen M11 G15 smart-HTTP claims or redefine M9.

## Supersedes / amends

- Supersedes M11 / M22 **Refuse-closed (terminal)** for FW-SRV-SSH / FW-SRV-GIT / FW-SRV-DUMB / FW-SRV-V2 in [[sot:root/arks-server]] and [[sot:root/production-residual-register]].
- Amends open questions in [[plan:root/20261108105446]] that froze “smart HTTP only.”
- Does **not** supersede [[evidence:root/20261108231300]] (G15 smart HTTP remains Claimed).
