# Structural conventions (always-on)

Owning SoT: `.spec/sot/structural-conventions.sot.md`.
Crate rings / proposed names: `.spec/sot/crate-architecture.sot.md`.
Product naming: `.spec/sot/product-naming.sot.md`.

## Must follow

1. **SRP** — one bounded responsibility per crate/primary module; split only for dependency direction, lib/bin target, or consumer isolation.
2. **Modularization** — inner→outer; pure logic separate from I/O; facades before backends; composition roots wire, they do not re-own domain logic.
3. **No flat listing** — do not dump unrelated crates, modules, or docs at one level without a parent grouping/ownership reason.
4. **Naming** — `arks` / `arks-<context>` kebab-case; no `git` substring in short names; Rust snake_case / PascalCase / SCREAMING_SNAKE_CASE per ecosystem norms.
5. **Context-based directories** — group by domain context; keep Specfold (`.spec/`), public docs (`docs/`), and future Cargo crate members in distinct roles.
6. **Anti-fragmentation** — no micro-crate or forwarding-only module per helper; one owning implementation per responsibility.
7. **Language-standard layouts** — prefer ecosystem norms. For Rust: Cargo workspace with conventional crates members first. Git / Specfold / public docs / `.agents/` are orthogonal overlays, not alien Rust trees.

## Cite (do not fork)

- `C:\workspaces\humanymous\.spec\sot\crate-architecture.sot.md`
- `C:\workspaces\humanymous\.spec\sot\rust-migration.sot.md` (Structure and naming)
- `C:\workspaces\humanymous\.spec\sot\structural-development.sot.md`
- `C:\workspaces\humanymous\.agents\rules\10-rust-conventions.md`

If a layout or split seems required but conflicts with rings/bottom-up/naming: **stop and ask** (rule `40-ambiguity-ask`).
