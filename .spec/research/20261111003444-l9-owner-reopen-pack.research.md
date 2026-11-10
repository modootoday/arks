---
id: research:root/20261111003444
title: L9 crates.io / docs.rs — owner reopen pack (blocked; no publish)
status: active
created: 2026-11-10T15:34:44Z
updated: 2026-11-10T15:34:44Z
tags: [l9, crates-io, docs-rs, owner-policy, reopen-pack, blocked]
verify: manual
rel:
  depends_on:
    - sot:root/public-surface
    - sot:root/product-naming
    - sot:root/program-roadmap
    - research:root/20261110085731
    - research:root/20261110095142
    - research:root/20261111001816
    - evidence:root/20261111003444
---

# L9 crates.io / docs.rs — owner reopen pack

## Status

**Blocked** — owner policy closed. This note is a **reopen checklist only**. Agents MUST NOT flip `publish = true`, mint trusted-publisher bindings, or run live `cargo publish` without an owner SoT amend that reopens L9.

## Why blocked (current tip)

| Item | State |
|------|-------|
| CLI crate `publish` | `false` on `crates/arks` (+ workspace crates) |
| Environment `crates` | Dry-run / OIDC wiring exists (`release.yml`); dry-run ≠ live PASS |
| Owner SoT reopen | **Absent** in-repo (no ADR/SoT flipping L9 to Open/Done) |
| Agent authority | None for unauthorized registry publish |

## Owner reopen checklist (when owner decides)

1. **SoT amend** — primary: [[sot:root/public-surface]] crates.io timing row + [[sot:root/program-roadmap]] tip table L9 → Open/Done authority; residual register row update.
2. **Name / metadata** — confirm crate name `arks` still free/owned; English README blurb; no secrets in package.
3. **`publish = true`** — flip only crates intended for crates.io (start with CLI or a documented subset); keep internal helpers `publish = false` unless intentional.
4. **Trusted publisher** — crates.io OIDC bound to `release.yml` + Environment `crates` (reviewers configured in GitHub UI).
5. **Dry-run** — `bash scripts/release-channel-readiness.sh` + Actions `release.yml` dry_run=true green.
6. **Tag** — annotated `v*` → dist + draft Release; approve Environment `crates` job.
7. **docs.rs** — follows publish; verify docs build; fix `package.metadata.docs.rs` if needed.
8. **Evidence** — file `.spec/evidence/` with crates.io version URL + docs.rs URL; flip tip table L9 **Done**.

## Explicit non-options

- Unauthorized `cargo publish` / token paste into repo.
- Agent invents L9 Done without owner SoT.
- Treating dry-run green as crates.io PASS.

## Cross-links

| Artifact | Role |
|----------|------|
| [[research:root/20261111001816]] | Per-gap unblock ledger |
| [[research:root/20261110085731]] | OIDC release research |
| `docs/release.md` | Operator dry-run / live steps |
| [[evidence:root/20261111003444]] | U-ladder pack carrying this reopen pack |
