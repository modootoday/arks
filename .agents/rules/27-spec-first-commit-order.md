# Spec-first commit order

When public Specfold **meaning** changes, land Spec commits **before** related implementation. Do not mix Spec canonization with crates / workflows / viz in one commit.

Owning index: `.spec/README.md` §Commit order. Router: `AGENTS.md` hard constraint **Commit order**. Process SoT: `.spec/sot/agent-governance.sot.md`.

## Must

1. **Meaning change in tracked `.spec/sot/**` (or classification README)** → own Spec commit(s) first.
2. **Then** code / stubs / harnesses / human docs that implement that meaning — separate stepwise commits.
3. One **concern** per commit (rule `91-commit-hygiene`).
4. Keep machine journey notes in **`.spec.local/`** (gitignored) — never promote local journey SoTs into tracked `.spec/` unless the owner explicitly asks.

## Must not

- Mix `.spec/sot/**` canonization with `crates/**`, `.github/workflows/**`, or HTML viz in the same commit.
- Commit code that depends on uncommitted Spec meaning drift.
- Treat local `.spec.local/sot/post-spec-canon-commit-order.sot.md` as public Claimed status (it is machine guidance only).

## Cite

- `.spec/README.md`
- skill `commit-governance`
- skill `design-sot` → skill `implement-sot-slice`
