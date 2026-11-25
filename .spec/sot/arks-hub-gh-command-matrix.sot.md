---
id: sot:root/arks-hub-gh-command-matrix
title: arks hub gh command matrix
status: active
created: 2026-11-10T19:06:00Z
updated: 2026-11-25T04:00:00Z
tags: [arks-hub, gh, matrix, arks-hub-cmd, cli-parity, native-api, survey, single-release]
verify: manual
rel:
  depends_on:
    - sot:root/arks-hub-product-intent
    - sot:root/arks-hub-cli-surface
    - plan:root/20261111040155
    - adr:root/20261119230000
    - sot:root/arks-self-host-server
    - sot:root/arks-cli-output-exit-contract
  evidence: []
  governs:
    - .spec/sot/arks-hub-gh-command-matrix.csv
---

# arks hub gh command matrix

Version **0.4.0**. Canonical **`gh` → `arks` (flat forge client) command matrix** so humans who already use GitHub CLI can keep muscle memory under the **single** published binary **`arks`**. Logical pillar name may still say “hub”; **taught argv is flat** (`arks pr …`) per [[sot:root/arks-hub-product-intent]] v0.4. CSV column `arks_hub_target` means **leaf path under `arks`** (historical name; `hub` prefix not required). **Surface parity ≠ subprocess.** Implementation MUST use a **native host API client** (host via `arks serve`); MUST **never** spawn `gh` or `git`.

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
| **out-of-scope** | gh / GitHub platform surface we will not mirror (extensions, Copilot, agent-task/skills, gh self-licenses, internal preview, **`auth setup-git`**, Actions `run`/`workflow`/`cache`/`secret`/`variable`, Codespaces, attestations) per [[adr:root/20261119230000]]; plus the hub-native `review` group (rule 12c) |

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

Decisions for the three hub-native rows (queue unit Q18, [[plan:root/20261124000000]]). Each follows
the scope test of [[adr:root/20261119230000]]: arks owns the primitive, gh supplies naming only.

12a. **`milestone` — v1, implemented.** Milestones are an arks-owned forge primitive (ADR decision 1).
    gh has no milestone command, so the argv is arks-defined in gh style (pflag rules, `-R/--repo`,
    gh export flags). A milestone is named by its number or its exact title.

    | Leaf | Argv | Effect and output |
    |------|------|-------------------|
    | `milestone list` (`ls`) | `[-R] [--state open\|closed\|all] [-L n] [--json f] [-q] [-t] [-w]` | Default state `open`, limit 30. Non-TTY tab-separated rows of number, title, state, description; `--json` fields `description`, `number`, `state`, `title`, `url` |
    | `milestone create` | `<title> \| --title <title> [-d/--description] [-R]` | Creates an open milestone; prints its URL to stdout |
    | `milestone edit` | `<milestone> [--title] [-d] [--state open\|closed] [-R]` | At least one change flag; prints the URL |
    | `milestone close` | `<milestone> [-R]` | Sets state `closed`; closing a closed milestone is a no-op success with a stderr note |
    | `milestone delete` | `<milestone> [--yes] [-R]` | Removes it; issues and pull requests that carried it are cleared |

    Bad argv and an invalid `--state` are usage failures (exit 2, or 1 under personality `gh`); an
    unknown milestone and forge errors exit 1; no token exits 4 ([[sot:root/arks-cli-output-exit-contract]]).
    The forge validates `state` (`open` or `closed`, HTTP 422 otherwise). The bare `milestone` group
    row is group help.
12b. **`serve` — v1, transitional listen/bind contract.** `arks hub serve` is the thin transitional
    alias of the forge module of `arks serve` ([[sot:root/arks-self-host-server]] item 14; rule 33):
    the same listener and store code as `arks serve --forge-listen`, nothing more. Argv:
    `arks hub serve [--listen HOST:PORT] [--data DIR]`.
    1. Default bind is loopback `127.0.0.1:3920` (forge port convention, self-host SoT item 26).
    2. An explicit non-loopback bind (`0.0.0.0:PORT`, a LAN interface, `[::]:PORT`) is honoured, because
       LAN / WAN reach is an in-scope operator choice (self-host SoT items 5 and 21). The listener
       prints a stderr notice that the transport is plain HTTP with no TLS and names the TLS paths
       (`arks serve` in-process TLS or a reverse proxy). This is the full listen/bind contract; it
       claims no TLS, no WAN threat model and no forge-in-serve depth.
    3. Every start prints a stderr note that `arks hub serve` is transitional and names
       `arks serve --no-http --no-ssh --forge-listen …` as the primary host command.
    4. Exit classes: a bad `--listen` value, a missing option value, an unknown option or a stray
       argument exit 2 before binding; a bind failure (address in use, not permitted) exits 1.
    No in-process TLS, live rebind or Git-root flags are added to the transitional alias; those stay
    `arks serve` flags.
12c. **`review` — refused, out of scope.** A top-level `review` group would be a second dialect for a
    primitive that `arks pr review` already carries with gh-identical argv (CSV row `pr review`, v1).
    The scope test admits a primitive once, under the gh name (ADR decisions 1 and 5; §Why: no
    parallel forge dialect). `arks hub review` refuses on stderr, naming `arks pr review`, makes no
    forge call, and exits 2 (usage / refuse class; 1 when the hub runs gh-faithful exits, matching
    gh's unknown-command exit). `review` is not added to the flat forge leaves, so bare `arks review`
    keeps the git-personality unknown-command path. Bringing it back needs a SoT amendment that
    defines a review primitive `pr review` cannot express.

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
9. No row is `later` since Q18 (rules 12a–12c): `milestone*` and `serve` are `v1`, `review` is
   `out-of-scope`. The living CSV carries no `Deferred` status.
