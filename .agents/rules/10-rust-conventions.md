# Rust conventions (cite, do not fork)

Authoritative Rust / English-prose / Cargo conventions live in **humanymous** — cite them; do not duplicate a competing ledger here.

## Live pointers

| Topic | Cite |
|-------|------|
| Project AGENTS + progressive disclosure | `C:\workspaces\humanymous\AGENTS.md` |
| Rust conventions rule | `C:\workspaces\humanymous\.agents\rules\10-rust-conventions.md` |
| Rust migration / toolchain SoT | `C:\workspaces\humanymous\.spec\sot\rust-migration.sot.md` |
| Bottom-up structural layers | `C:\workspaces\humanymous\.spec\sot\structural-development.sot.md` |

## arks deltas (thin)

- Product / crate / bin name is **`arks`** (no literal `git` in short name) — `.spec/sot/product-naming.sot.md`.
- English-only lock for all authored prose: `.spec/sot/language-english-only.sot.md` and rule `05-english-only` (cites humanymous; do not fork).
- Structural / SRP / layout lock: `.spec/sot/structural-conventions.sot.md` and rule `15-structural-conventions` (Cargo workspace / crates members first; Specfold, docs, agents overlays).
- Prefer standard Cargo workspace layout per crate-architecture + structural-conventions; keep pure parsers/format logic separate from I/O and CLI.
- Do not introduce foreign executable script strings as a substitute for Rust implementation of core Git-compat logic.
