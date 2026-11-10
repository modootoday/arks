# Bottom-up build order (inner modules first)

Implement and verify **inside-out**. Do not claim outer product success from unproven inner modules.

## Pattern cite (do not copy ledger)

General layer / promotion discipline: `C:\workspaces\humanymous\.spec\sot\structural-development.sot.md` (bottom-up implementation; unit evidence does not promote product).

## arks order (normative intent)

Whole-product phase letters are owned by `.spec/plan/20261108082148-development-build-order.plan.md`:
**C → S → R → P → L → H → Pa → Pb → Pc → E** (+ post-E program ladder). Program milestones **M0–M24**, historical **CLI-parity production = M9**, lease capability = M10, production-server = M11, active alias = M16, full journeys = M17, catalog Deferred=0 = M21, L3 Claimed shape = M23, CELIR index dual-path = M24: `.spec/sot/program-roadmap.sot.md` and staged plans under `.spec/plan/*-m*.plan.md` / `*-program-milestones*.plan.md` (see roadmap `rel.depends_on`). Crate import edges follow `.spec/sot/crate-architecture.sot.md` (inner → outer). Core crate slice detail: `.spec/plan/20261108082110-core-build-slice.plan.md`. When packaging detail and phase/milestone exit criteria disagree, **phase/milestone exits** win for product claims; **crate rings** win for dependency direction.

Prefer this direction unless an owning SoT says otherwise:

1. **Contracts / SoT** — accepted requirements and non-coverage before code.
2. **C — Core object kernels** — hash/OID, loose objects (and fixtures) before later rings.
3. **S — Storage adapter (Git-first)** — object-store facade + filesystem backend after object kernels; CELIR only as optional index-only `celir-index` (rule `50-celir-boundary`).
4. **R — Refs + index** — after the store facade so persistence goes through one adapter.
5. **P — Pack + protocol client** — pack/idx and pkt-line / fetch-pack / push helpers after refs/reachability.
6. **L — Local CLI foundation** — Wave L composition; not production.
7. **H — Remote GitHub-class client** — after local foundation; not production.
8. **Pa / Pb / Pc — Porcelain parity waves** — toward Git CLI symmetric inventory; **Pc (M9) = CLI-parity production**.
9. **E — Lease features** — after CLI-parity production unless a lease SoT explicitly scopes local-only earlier; L3 claimed shape ships **M23**.
10. **Post-E program ladder** — M11 server, M12–M22 residual/catalog closes, M24 CELIR index dual-path — per roadmap; do not redefine frozen M9–M16 exits.
11. **Clean-room product gates** — last mile for “verified” (rule `60-clean-room-verify`); host greens stay advisory. Gates **G0–G27** (+G28 when present) sequencing: `.spec/sot/interop-test-matrix.sot.md`.

## Promotion rules

- Lower-layer unit green ≠ protocol green ≠ clean-room product green.
- Consumers may stub against a frozen typed API, but must not advertise product verification early.
- Do not call M5/M6 “production.”
- Skill `implement-sot-slice` follows this order.
