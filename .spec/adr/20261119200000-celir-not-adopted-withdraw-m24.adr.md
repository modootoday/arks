---
id: adr:root/20261119200000
title: CELIR not adopted — withdraw index-only dual-path (M24, G22–G25)
status: active
created: 2026-11-19T20:00:00Z
updated: 2026-11-19T20:00:00Z
tags: [celir, withdrawn, m24, g22, g23, g24, g25, storage, build-hygiene, decision]
rel:
  supersedes:
    - adr:root/20261110052030
  depends_on:
    - sot:root/storage-celir-boundary
    - sot:root/cloud-agent-dropin
    - plan:root/20261119180000
---

# CELIR not adopted — withdraw index-only dual-path (M24, G22–G25)

## Context

CELIR (`discrete-circuit` packages `celir-index`, `celir-index-runtime`, `celir-data`) is an
experimental package. [[adr:root/20261110052030]] allowed it into the product Cargo graph as an
optional, default-OFF, index-only feature on `arks-store`, shipped as **M24** with Docker gates
**G22–G25**. Because Cargo resolves optional path dependencies even when the feature is off, every
fresh clone needed the sibling `discrete-circuit` tree (or CI stubs) to build at all
([[sot:root/cloud-agent-dropin]] §B).

The owner decided not to adopt CELIR.

## Decision

1. **Not adopted.** CELIR MUST NOT appear in the arks Cargo graph in any form (no feature, no path
   dependency, no out-of-workspace companion crate). The `arks-store` feature `celir-index`, module
   `celir_index`, and the `arks-lease` feature `celir-index-stress` are removed.
2. **Withdrawn milestones and gates.** **M24** (CELIR index dual-path) and gates **G22**
   (`celir-index-contention`), **G23** (`celir-dual-path-git`), **G24** (`l3-celir-index-joint`),
   **G25** (`celir-index-scale`) are **Withdrawn**: their scripts, compose services and gate trees are
   removed; their matrices remain as historical records marked Withdrawn.
3. **History kept.** Evidence receipts under `.spec/evidence/` and historical plans / research /
   ADRs are retained unchanged as history; they no longer describe current product scope.
4. **Unaffected.** Filesystem `.git` remains the sole object store (CELIR ObjectStore was already
   Refuse). **M23** L3 claimed shape and **M26 / G34** shared-index N-way do not depend on CELIR and
   stay as recorded.
5. **Build hygiene.** Default and all-feature builds MUST NOT require any sibling tree; the CI
   discrete-circuit stub script and compose mounts are removed.

## Consequences

The SoTs [[sot:root/celir-index-dual-path]], [[sot:root/celir-index-contention-clean-room]],
[[sot:root/celir-index-scale-clean-room]], [[sot:root/celir-dual-path-git-verified]] and
[[sot:root/l3-celir-index-joint-clean-room]] become **superseded**. [[sot:root/storage-celir-boundary]]
reduces to "CELIR not adopted". Remaining historical mentions elsewhere in Specfold are read through
this ADR.
