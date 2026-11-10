---
id: plan:root/20261108082148
title: Development build order
status: active
created: 2026-11-08T08:21:48Z
updated: 2026-11-09T02:52:00Z
tags: [build-order, bottom-up, phases, arks, cli-parity, server, concurrency, issue-resolution, perf]
rel:
  depends_on:
    - sot:root/product-intent
    - sot:root/crate-architecture
    - sot:root/core-object-model
    - sot:root/storage-celir-boundary
    - sot:root/lease-concurrency-intent
    - sot:root/cli-surface
    - sot:root/interop-test-matrix
    - sot:root/product-verification-clean-room
    - sot:root/git-compat-surface
    - sot:root/remote-host-compatibility-surface
    - sot:root/program-roadmap
    - plan:root/20261108082110
    - sot:root/agent-dev-readiness
    - sot:root/product-naming
    - research:root/20261108073650
    - research:root/20261108183800
---

# Development build order

## Outcome

arks develops and verifies **bottom-up / inner-to-outer**: core kernels → storage facade → refs/index → pack/protocol → local CLI foundation → generic remote **client** (GitHub-class verify) → porcelain parity waves (**Pa→Pb→Pc**) → **CLI-parity production (M9 / Pc)** → then parallel **default lease capability (M10 / E**; high-concurrency **opt-in**) and **Git server (M11 / Sv / production-server)** → post-production **concurrency issue-resolution (M12 / Cm → M13 / Cd)** with clean-room git-vs-arks scenario compare (**G17/G18**) → **future comparative-perf unlock (M14 / Pf / G19 ≥20%)**. Outer layers do not claim success on unproven inner slices. [[plan:root/20261108082110]] remains the detailed **core** slice plan. Program milestones: [[sot:root/program-roadmap]].

## Authority

| Concern | Authority |
|---------|-----------|
| Product goals / non-goals | [[sot:root/product-intent]] |
| Crate graph / dependency direction | [[sot:root/crate-architecture]] |
| Object/ref/index identity model | [[sot:root/core-object-model]] + [[sot:root/git-compat-surface]] |
| CELIR / storage readiness | [[sot:root/storage-celir-boundary]] |
| Lease product vs Git locks | [[sot:root/lease-concurrency-intent]] |
| CLI symmetric parity / waves | [[sot:root/cli-surface]] |
| Interop gate unlock order | [[sot:root/interop-test-matrix]] |
| Verified venue | [[sot:root/product-verification-clean-room]] |
| CLI-parity production exit | [[sot:root/program-roadmap]] (**M9**) |
| Production-server exit | [[sot:root/program-roadmap]] (**M11**) |
| Issue-resolution exits | [[sot:root/program-roadmap]] (**M12/G17**, **M13/G18**) — Done = clean-room per-scenario git pathology vs arks resolved |
| Comparative-perf unlock | [[sot:root/program-roadmap]] (**M14/G19**) — separate from G17/G18 |
| Remote client + server (A)/(B) | [[sot:root/remote-host-compatibility-surface]] |
| Agent process / session light rules | `AGENTS.md`, `.agents/rules/70-bottom-up-build-order.md` |
| Phase reorder | Requires updating this plan + interop matrix + roadmap; chat-only reorder is non-binding |

## Phases

Principle: **each phase consumes only prior phase exits**.

| Phase | Name | Inputs | Outputs | Verification (clean-room intent) | Exit criteria |
|-------|------|--------|---------|----------------------------------|---------------|
| **C** | Core object kernels | core-object-model, git-compat; [[plan:root/20261108082110]] | OID, loose object r/w | G0, G1 | Stock Git ↔ arks blob OID parity |
| **S** | Storage adapter (Git-first) | C; storage-celir-boundary | Object-store trait + FS backend | G10 objects-subset; G1 | All object I/O via adapter; CELIR gated |
| **R** | Refs + index | C/S; refs/index research | Ref store; index; HEAD; repo | G2; index r/w portion | Stock Git reads refs/index; L0 locks |
| **P** | Pack + protocol client | R; pack/protocol research | Pack/idx; pkt-line; client libs | G4, G5; **gix-exit** | Pack interchange; offline fixtures; zero gix |
| **L** | Local CLI foundation | P + cli-surface Wave L | `arks` Wave L commands | G6; **full G3**; G1–G4 | Help honest; **not production** |
| **H** | Remote client (generic; GitHub-class verify) | L + P; remote-host (A) client | remote/fetch/push/clone | G7; G8/G9 if claimed | Remotes green; **not production** |
| **Pa** | Porcelain parity A | H + cli-surface Wave Pa | Branching/diff/merge foundations | **G12** | Pa checklist; refuse closed |
| **Pb** | Porcelain parity B | Pa + Wave Pb | rebase/cherry-pick/bisect/… | **G13** | Pb checklist |
| **Pc** | Porcelain parity C + closure (**CLI-parity production**) | Pb + inventory | Full inventory − exclusions | **G14**; G0–G7 + G12–G13 | **M9 CLI-parity production** |
| **E** | Default lease capability (+ optional high-concurrency) | Pc default; lease SoT | L1/L2 lease APIs; L3 opt-in refuse-closed | G11 + CLI-parity regression | After M9; lease ≠ skip Git locks; N-way not forced |
| **Sv** | Git server (**production-server**) | Pc default; remote-host server | upload-pack / receive-pack + transports | **G15**; G5/G7 + G14 regression | **M11 production-server** |
| **Cm** | Concurrency issue-resolution — compat | Pc + residual worktree/submodule Done; catalog | Mitigate claim-set + G17 harness | **G17** per claimed ID | Every claimed ID: git pathology + arks resolved in Docker |
| **Cd** | Concurrency issue-resolution — differentiate | E + Cm; lease SoT | Differentiate claim-set + lease uplift | **G18** per claimed ID | Same Done rule; L3 still refuse-closed unless ADR |
| **Pf** | Comparative performance unlock (**future**) | Cm default; Cd recommended | ≥20% allow-list | **G19** | Separate from G17/G18; not silent M9–M13 raise |

### Phase notes

- **C→S→R→P** unchanged bottom-up engine order.
- **L/H are delivery waves**, not production.
- **Pa→Pb→Pc** close Git CLI symmetric inventory; only **Pc** is CLI-parity production.
- **E after Pc** by default (M10 after M9).
- **Sv after Pc** by default (M11 after M9); **E and Sv MAY run in parallel**.
- **Cm after Pc** (+ residual Done); **Cd after E+Cm**; **Pf strictly after Cm and Cd both exit** — perf/timing speed optimization MUST NOT run as parallel Done with open issue-resolution.
- **Cm/Cd Done ≠ SoT claim** — requires G17/G18 clean-room issue-repro compare per claimed catalog ID.
- Parallelism: docs/fixtures/stubs MAY proceed; qualification does not. Perf-opt engineering waves are **not** qualified stubs for Cm/Cd.

### Suggested commands (intent)

```text
docker compose -f verify/compose.yaml run --rm gate-g<N>-<slug>
cargo test -p <crate>   # advisory on host
spec check
```

## Rollback

| Phase | Rollback |
|-------|----------|
| C–P | Feature-flag / drop outer crates; keep SoTs |
| L–H | Hide CLI commands / feature-flag remotes |
| Pa–Pc | Refuse newly claimed commands; keep prior wave greens |
| E | Disable lease / high-concurrency flags; Git-compat CLI-parity mode unaffected |
| Cm–Cd | Shrink claimed scenario set; keep prior G17/G18 receipts; do not invent host-only Done |
| Pf | Revert G19 to record-only G16; keep issue-resolution receipts intact |

## Record

- 2026-11-08: Plan created (v0.2 production=M6).
- 2026-11-08: Restructured for CLI-parity production=M9; phases Pa/Pb/Pc added.
- 2026-11-09: Added Cm/Cd/Pf for M12–M14; G17/G18 issue-repro Done rule; G19 future ≥20%.
