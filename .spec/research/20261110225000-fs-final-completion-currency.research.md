---
id: research:root/20261110225000
title: FS_final completion currency — clean-room re-stamp + public-surface honesty
status: active
created: 2026-11-10T13:50:00Z
updated: 2026-11-10T13:50:00Z
tags: [fs-final, completion, clean-room, docker, git-vs-arks, public-surface, honesty]
verify: manual
rel:
  depends_on:
    - research:root/20261110212414
    - sot:root/public-surface
    - sot:root/production-residual-register
    - sot:root/interop-test-matrix
    - evidence:root/20261110224814
---

# FS_final completion currency

## Question

After tip ≥ **`54c9dc2`** (OBJ-004 sha1cd Claimed; FS-007/WIN-003 advisory), can maintainers say the product is **fully complete** for the **git feature symmetry DoD**, and what remains documented Impossible / advisory / owner-policy?

## Verdict (honesty bar)

| Scope | Answer |
|-------|--------|
| Git feature symmetry DoD (**FS0–FS_final**) | **YES** — Claimed rows have carrying evidence; no casual N/A for real git features |
| “Everything” including live G8 latency SLO, crates.io publish timing (**L9**), absolute crypto immunity, forge UX, full cloud-sync safety | **NO** — documented non-goals |
| Overall one-word for operators asking “fully complete now?” | **PARTIAL** — FS_final **YES**; non-FS residuals remain Impossible/advisory/owner-policy |

## Method

1. Inventory compose services + suite entrypoints (L8 `run-l008-product-completion-e2e.sh`, S6 `run-s006-symmetry-e2e.sh`, G0–G41, fs007 advisory, g008-live-forge-smoke local-forge / live / scale).
2. Sequential Docker-only re-stamp under `.spec/evidence/20261110224814-full-cleanroom-matrix/` (halt OOM/fail; mkdir out dirs; no `$home` trap).
3. Secrets-absent live gates recorded **SKIP** with ADR reason (never invent PASS).
4. Git↔arks comparison: `docs/git-vs-arks.md` + residual register currency; fix real gaps found during re-stamp before claiming green.
5. Refresh public surface (README, CONTRIBUTING, docs/*) for Claimed / advisory / Impossible.

## Gaps found and fixed during re-stamp

- **G20** `M15-FS5-03` still asserted historical “exclude” substrings after **FS1** editor-required honesty landed. Updated clean-room test + `matrix-g020-cli-symmetric-parity.md` + historical M15 SoT note to accept EDITOR / SEQUENCE_EDITOR fail-closed strings. Re-PASS in Docker.
- **G11** / **G11-L3** / **G18** still grepped full `cargo metadata` for optional `celir-*` path-deps (false FAIL when `discrete-circuit` is mounted). Aligned with G20/G15 honesty: default `cargo tree -p arks-store --edges normal` must stay CELIR-free; parse-time optional deps alone are not a product Claimed edge.
- **G15** capability honesty still asserted `!push-options` after **O4** Claimed advertise+forward. Updated clean-room twin to require `push-options` (matches `arks-server` unit tests). Re-PASS in Docker.
- **G19** tip remasure (real gate FAIL exit 1, **not** OOM/137): M17 ≤0.8× **19/19 PASS**; PF5 ≤0.5× miss `J-ALIAS-01` (0.5192 process-tax) + `J-REVERT-01` (0.6667 short-wall). **No remasure thrash.** Recorded `aggressive_na` + eval-only PASS; prior primary 8/8 at `79fc9bf` remains archival.
- **G19** tip remasure after sha1cd: M17 ≤0.8× **19/19**; ≤0.5× miss **J-ALIAS-01** (0.5192 process-tax) + **J-REVERT-01** (0.6667 short-wall). Residual amortize helped ALIAS 0.56→0.52 but did not clear 0.5 — recorded `aggressive_na` (no thrash); umbrella **Claimed(partial)** on tip; historical **8/8** at `79fc9bf` retained.

## Documented non-goals (not FS gaps)

- Absolute SHA-1 collision **immunity** (detect+refuse Claimed; immunity Impossible)
- Forge Checks / merge-queue / Issues UX (use `gh`)
- Embedded forge OAuth browser UX
- Full multi-device cloud-sync safety on `.git` (FS-007 / WIN-003 **advisory** detect only)
- Live G8 / G8-live-scale **latency PASS** without secrets
- **L9** crates.io / docs.rs publish timing (owner policy)

## Public surface refreshed

README, CONTRIBUTING, `docs/limits.md`, `docs/git-vs-arks.md`, `docs/workflows.md`, `docs/contribute.md`, `docs/index.md`, `sot:root/public-surface` honesty rule 32.

## Evidence

[[evidence:root/20261110224814]] — full clean-room matrix results + gate logs.
