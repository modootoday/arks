# CLI personality dispatch (git | gh | arks)

Single published binary **`arks`**. Personalities are env / argv0 / shim — not extra crates.io releases.

Owning SoT: `.spec/sot/arks-cli-personality-factory.sot.md` (design / Partial — do not invent Claimed depth). Output/exit: `.spec/sot/arks-cli-output-exit-contract.sot.md`.

## Must

- Personalities: **`git` | `gh` | `arks`**.
- Resolution order (first wins): `ARKS_PERSONALITY` → argv0/shim basename → default **`arks`**.
- **`git` / `gh` personalities:** peer-pure stdout + peer-native exit codes (CI drop-in).
- **Default `arks` personality:** git-first merged surface; OverlapFacade input priority **git (0) → gh (1)**; unified combined output for overlap leaves unless single-peer flags opt out.
- Keep factory/Registry/OverlapFacade/ExitCodePolicy changes Spec-first when meaning changes (rule `27`).

## Must not

- Ship separate product binaries `arks-hub` / `arks-daemon` / `arks-agent` / `git` / `gh`.
- Treat personality SoT design as Claimed forge/SSH depth (rule `28`).
- Spawn stock `git`/`gh` to fake personality (rule `30`).
- Teach nested `arks hub …` as the primary day-to-day forge noun (transitional alias only — rule `33`).

## Cite

- skill `guard-cli-surface`
- role `.agents/agents/cli-surface.md`
