---
id: plan:root/20261111040155
title: arks-hub internal crate + arks hub dispatch
status: active
created: 2026-11-10T19:01:55Z
updated: 2026-11-10T19:16:00Z
tags: [arks-hub, crate, skeleton, workspace, octocrab, native-api, cli, gh-symmetry, matrix, single-release]
verify: manual
rel:
  depends_on:
    - sot:root/arks-hub-product-intent
    - sot:root/arks-hub-cli-surface
    - sot:root/arks-hub-gh-command-matrix
    - sot:root/crate-architecture
    - sot:root/product-intent
    - sot:root/program-roadmap
  implements:
    - sot:root/arks-hub-product-intent
---

# arks-hub internal crate + `arks hub` dispatch

## Goal

Design-first home for forge UX as **`arks hub …`** on the **single** published binary **`arks`**. Internal library crate **`arks-hub`**; **native API only** (never wrap `gh`/`git`). SoT + **exhaustive gh matrix** are the primary deliverable; `arks hub --help` stub OK.

## Directive (normative)

1. Crate `crates/arks-hub` is a **library** (not a published product bin).
2. Workspace `members` include hub; `arks` (bin package) depends on it and dispatches the **`hub`** command group.
3. **Do not** ship / advertise a separate `arks-hub` binary or crates.io release.
4. Hub HTTP via native client (`octocrab` / equivalent). **Forbidden:** spawn `gh` or `git`.
5. CLI names follow [[sot:root/arks-hub-gh-command-matrix]] (v1 rows first) under `arks hub …`.
6. Later: optional `arks hub serve`; `.git`-native reads via in-process libs only.
7. No Claimed until evidence.

## Layout

```text
crates/arks-hub/
  Cargo.toml          # lib only (or lib + private test bin — not product release)
  src/lib.rs
crates/arks/src/…     # dispatch: arks hub → arks_hub::…
.spec/sot/arks-hub-gh-command-matrix.csv
.spec/sot/arks-hub-gh-command-matrix.sot.md
```

## Acceptance

| Check | Done when |
|-------|-----------|
| SoTs | product-intent, cli-surface, **gh-command-matrix** (single-`arks` wording) |
| Matrix coverage | pinned `gh help reference` leaves present in CSV |
| Stub (optional) | `arks hub --help` lists alias-friendly groups; `cargo build -p arks` |
| Boundary | no gh/git spawn; no second published bin |
| Honesty | no Claimed forge |

## Non-goals

Full PR/issue implementation; web UI; separate `arks-hub` release; `gh` thin backend; requiring hub for M9/M16 Git DoD.

## Follow-on

1. Implement matrix **v1** auth + `pr list/view` against live API + evidence.
2. Amend [[sot:root/crate-architecture]] when lib lands on main.
3. Optional CI build via `-p arks` (pulls hub) or `-p arks-hub` lib check.
4. Re-survey when bumping `gh_pin`.
