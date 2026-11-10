---
name: implement-sot-slice
description: >
  Use when implementing an accepted SoT/plan slice into packages and tests,
  bottom-up (inner modules first): wire a feature or vertical slice after design
  is ready. Not for design-only requests or explanations without code changes.
---

# Implement SoT slice (bottom-up)

## Preconditions

- Accepted SoT/plan (or user-pasted design).
- Read rule `70-bottom-up-build-order` and rule `10-rust-conventions` (humanymous cite).
- Confirm CELIR/lease work is in-scope via its own SoT (else rule `50-celir-boundary`). CELIR product path is index-only `celir-index` (default OFF) — never ObjectStore.

## Steps

1. `spec context <path>`; map requirements to the innermost module touched.
2. Implement **inner first**: pure format/parser/state + unit tests before protocol, CLI, or product compose.
3. Do not invent Git semantics — every byte/capability claim needs a primary citation path via `git-compat-surface` / research.
4. Wire outer layers only after inner fixtures/tests exist for the claimed subset.
5. Run scoped Cargo test/fmt/clippy when code exists; host green is advisory for product claims.
6. Product “verified” → skill `verify-clean-room` (rule `60-clean-room-verify`).
7. `spec check` for touched documents.

## Gotchas

- CLI/porcelain must not advertise unsupported pack/ref/protocol behavior.
- Lower-layer unit pass ≠ clean-room product pass.
- Pattern cite (do not copy ledger): `C:\workspaces\humanymous\.spec\sot\structural-development.sot.md` and humanymous skill `implement-sot-slice`.

## Done checklist

- [ ] Behavior covered by tests appropriate to the layer
- [ ] No always-on folklore dumped into root AGENTS.md
- [ ] Boundary rules (`50`, `60`, `70`) respected
