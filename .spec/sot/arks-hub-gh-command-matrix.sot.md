---
id: sot:root/arks-hub-gh-command-matrix
title: arks hub gh command matrix
status: active
created: 2026-11-10T19:06:00Z
updated: 2026-11-20T03:00:00Z
tags: [arks-hub, gh, matrix, arks-hub-cmd, cli-parity, native-api, survey, single-release]
verify: manual
rel:
  depends_on:
    - sot:root/arks-hub-product-intent
    - sot:root/arks-hub-cli-surface
    - plan:root/20261111040155
    - adr:root/20261119230000
  evidence: []
  governs:
    - .spec/sot/arks-hub-gh-command-matrix.csv
---

# arks hub gh command matrix

Version **0.3.1**. Canonical **`gh` → `arks` (flat forge client) command matrix** so humans who already use GitHub CLI can keep muscle memory under the **single** published binary **`arks`**. Logical pillar name may still say “hub”; **taught argv is flat** (`arks pr …`) per [[sot:root/arks-hub-product-intent]] v0.4. CSV column `arks_hub_target` means **leaf path under `arks`** (historical name; `hub` prefix not required). **Surface parity ≠ subprocess.** Implementation MUST use a **native host API client** (host via `arks serve`); MUST **never** spawn `gh` or `git`.

**Survey pin:** host `gh` **2.103.3** (2026-10-26). Captured via `gh help` + `gh help reference` on Windows. Cross-OS: help text is the same CLI contract; re-run survey when advancing the pin. Prior pin **2.103.0** superseded for inventory currency — CSV `gh_pin` column MUST match this survey pin (sibling refresh).

**Host vs clean-room:** hub muscle-memory inventory is **gh-surveyed** (not Docker git pin). Clean-room Git/daemon judge remains stock **2.39.5**; host Git **2.56.0.windows.4** is skew-only for git/daemon pillars ([[sot:root/verify-cleanroom-symmetry-matrix]]).

**Ledger file:** `.spec/sot/arks-hub-gh-command-matrix.csv` (columns below). Column `arks_hub_target` means the path **after** `arks hub` (e.g. `pr merge` → `arks hub pr merge`).

## Rule

### Columns

| Column | Meaning |
|--------|---------|
| `gh_pin` | Surveyed `gh` version |
| `gh_command` | `gh` command path (`pr merge`, `auth setup-git`, …); `(none)` = hub-native with no gh peer |
| `arks_hub_target` | Intended path under `arks hub …`; `OUT` = not mapped |
| `phase` | `v1` must-have · `later` · `out-of-scope` |
| `notes` | Alias / API / divergence notes |

### Phase meanings

| Phase | Normative meaning |
|-------|-------------------|
| **v1** | Required for a useful **gh-symmetric** collaboration slice (auth + PR/issue/release/repo/label + search/status/api/config/alias/completion). Still **not Claimed** until implemented + evidence |
| **later** | Keep gh-shaped names; defer after v1 |
| **out-of-scope** | gh / GitHub platform surface we will not mirror (extensions, Copilot, agent-task/skills, gh self-licenses, internal preview, **`auth setup-git`**, Actions `run`/`workflow`/`cache`/`secret`/`variable`, Codespaces, attestations) per [[adr:root/20261119230000]] |

### Alias law (normative)

1. Prefer **identical** top-level and subcommand names to `gh` where the row is not `OUT` (`pr`, `issue`, `auth`, `repo`, `release`, `label`, `api`, …), invoked as **`arks hub <path>`**.
2. `co` → `pr checkout` alias MUST exist when `pr checkout` is claimed (match gh).
3. Muscle-memory mapping is for **forge UX**, not for `git` porcelain. Core [[sot:root/cli-surface]] remains `git`→`arks`. Optional user shell functions MAY forward `gh`→`arks hub`; that is **not** a second product binary.
4. **Never** implement a row by wrapping `gh` or `git`.
5. **Single release:** no published `arks-hub` bin; CSV targets are command-group paths only.

### Host divergence (normative)

6. CLI **shape** tracks `gh` for muscle memory.
7. Wire protocol is the **arks forge API** served by `arks serve` (first verify peer). For arks-owned primitives it uses GitHub-shaped REST paths and field names, so the same argv MAY target a GitHub-class host via `--host` for that compatible subset only. GraphQL is not required ([[adr:root/20261119230000]]).
8. Scope test: a row is in scope when its function can run on an independent host (not a GitHub-exclusive service). GitHub-platform-only rows (Actions, Codespaces, Copilot, attestations, gh extensions, agent-task) are `out-of-scope`; in-scope rows not yet implemented are `later`. Do not bring either back without a SoT amendment that defines an arks-owned primitive.

### Dangerous gh overlaps with core git

9. `pr merge` ≠ `arks merge`. Users type `arks hub pr merge` (host API). Git merges stay `arks merge`.
10. `pr checkout` / `repo clone` / `gist clone` MUST NOT spawn `git`. Resolve via API + in-process arks library and/or print suggested `arks …` commands.
11. `auth setup-git` is **out-of-scope** (teaches git to call gh).

### Hub-native extras

12. Rows with `gh_command=(none)` (`serve`, top-level `milestone`, optional `review` group) are **additive**; they do not break gh symmetry for mapped names. **`serve`:** transitional `arks hub serve` MAY remain as a thin alias; primary host start is `arks serve` ([[sot:root/arks-self-host-server]]). Other hub-native rows: invoke as `arks hub …`.

### Survey refresh

13. When bumping `gh_pin`, re-dump `gh help reference`, diff CSV, and amend this SoT before claiming symmetry completeness against the new pin.

## Why

Exhaustive gh inventory prevents inventing a parallel forge dialect. Explicit phases keep v1 honest. Absolute no-subprocess law keeps hub a real product, not a `gh` wrapper. Single-`arks` shipping keeps release honest.

## Clean-room DoD

Pillar 3 under [[sot:root/verify-cleanroom-symmetry-matrix]]: compose `dod-hub-matrix` → `verify/scripts/run-hub-gh-arks-matrix-e2e.sh`. Local multi-agent framework: [[sot:root/arks-hub-local-multi-agent-pr]]. Optional status ledger: `verify/matrices/hub-gh-arks-e2e-matrix.csv`. Default smoke ≠ Claimed; `ARKS_DOD_STRICT=1` fails closed.

## Verification

1. CSV row count and phases documented here match the file on disk.
2. Every `### gh …` leaf from the pinned `gh help reference` appears as a `gh_command` (coverage check).
3. No `v1`/`later` row may be implemented via `Command::new("gh"|"git")`.
4. Do not mark matrix rows Claimed from this SoT alone.
5. Public docs describe `arks hub`, not a separate `arks-hub` release.
6. Umbrella + scenario SoTs stay linked for clean-room public-story DoD.
7. Flag level (rung A4 of [[plan:root/20261120000000]]): `verify/scripts/run-gh-argv-inventory.py`
   reads each in-scope row's flags from the installed stock `gh <cmd> --help` (oracle only; leaves
   newer than the installed gh come from `verify/matrices/gh-argv-supplement.csv`) and runs them
   through `arks`; every flag must be ACCEPTED and every leaf must refuse an unknown flag. Host runs
   are advisory; the oracle version is recorded with each run (skew against the pin is expected).
8. `attestation*` and `codespace*` rows are `out-of-scope` (GitHub-platform-only,
   [[adr:root/20261119230000]] decision 3); the formerly `later` in-scope rows are `v1` since A4.
