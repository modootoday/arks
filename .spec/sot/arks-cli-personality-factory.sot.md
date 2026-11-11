---
id: sot:root/arks-cli-personality-factory
title: arks CLI personality dispatch and CommandLayer factory
status: active
created: 2026-11-10T20:15:00Z
updated: 2026-11-11T00:40:00Z
tags: [cli, personality, factory, command-layer, registry, overlap, exit-code, arks, git, gh, serve, daemon, design, partial]
verify: manual
rel:
  depends_on:
    - sot:root/cli-surface
    - sot:root/arks-cli-output-exit-contract
    - sot:root/arks-hub-product-intent
    - sot:root/arks-hub-cli-surface
    - sot:root/arks-hub-gh-command-matrix
    - sot:root/arks-self-host-server
    - sot:root/arks-daemon-product-intent
    - sot:root/arks-agent-cli-surface
    - sot:root/arks-partial-residual-wave
    - research:root/20261111093543
    - sot:root/product-naming
    - sot:root/crate-architecture
    - sot:root/public-surface
  evidence: []
---

# arks CLI personality dispatch and CommandLayer factory

Document revision **0.1.2** (SoT edit counter only — **not** a product/crate release). Normative **design** for (1) **personality dispatch** on the single published binary `arks`, (2) lean **CommandLayer + Registry** factory in `crates/arks`, and (3) **overlap** input/output/**exit-code** rules for the native `arks` personality.

**Honesty:** This SoT is **design intent**. Deep forge/SSH bodies remain **Partial / not Claimed**. CLI **skeleton** (PersonalityResolver + Registry + OverlapFacade + ExitCodePolicy + stubs) MAY land without inventing Claimed feature depth. **IW-PERSONALITY** ([[sot:root/arks-partial-residual-wave]]): ExitCodePolicy OR-of-failure unit greens **MUST NOT** invent `git`/`gh` drop-in Claimed — peer stdout/exit evidence required ([[research:root/20261111093543]], `gh` exit-codes 0/1/2/4). Normative stdout/stderr/exit **classes**: [[sot:root/arks-cli-output-exit-contract]]. Document revision ≠ crates.io/`arks` version bump.

Cross-owners: porcelain inventory [[sot:root/cli-surface]]; output/exit contract [[sot:root/arks-cli-output-exit-contract]]; forge client [[sot:root/arks-hub-product-intent]] / [[sot:root/arks-hub-cli-surface]]; host [[sot:root/arks-self-host-server]]; daemon [[sot:root/arks-daemon-product-intent]]; agent [[sot:root/arks-agent-cli-surface]].

## Rule

### Single binary

1. **Published binary is `arks` only.** No second release bin named `git` or `gh` ([[sot:root/product-naming]]). Drop-in is alias / PATH shim / env personality — not a rename of the crate.

### Personality dispatch (normative design)

2. **Personalities:** `git` | `gh` | `arks`.
3. **Resolution order (first wins):**
   1. Explicit `ARKS_PERSONALITY=git|gh|arks` (case-insensitive; unknown → fail-closed usage error).
   2. Else argv0 / shim basename: `git` / `git.exe` → `git`; `gh` / `gh.exe` → `gh`; `arks` / `arks.exe` → `arks`.
   3. Else default **`arks`**.
4. **Shell / Windows:**
   - `alias git=arks` (or PATH shim whose basename is `git`) → **git** personality.
   - `alias gh=arks` (or basename `gh`) → **gh** personality.
   - Bare `arks` → **arks** personality (git-first merged surface below — **not** gh-first).
5. **Peer-pure personalities:**
   - **`git`:** Git porcelain/plumbing argv + **git stdout/stderr shape** + **git exit-code semantics**. No forge unified sections. No requirement to emit forge client output.
   - **`gh`:** Surveyed `gh` leaf grammar + **gh stdout/stderr shape** + **gh exit-code semantics**. No git porcelain unless a surveyed `gh` leaf already owns that noun.
6. **CI / automation (normative guidance):** Scripts that today call `git` / `gh` SHOULD use **git** / **gh** personalities (alias, shim, or `ARKS_PERSONALITY`) so they receive **peer-native stdout and peer-native exit codes** — drop-in compatibility. The native **`arks` unified** path is for humans and for arks-native automation that **opts into** unified semantics (or forces a single peer via flags). **`arks` unified ≠ CI drop-in** unless flags force a single-peer mode (`--git-only`, `--forge-only`, `--porcelain`, …).

### Default `arks` personality = git-first merged surface

7. Bare `arks` is **git-first**, not gh-first: everyday bare status/config/porcelain parse as Git when the invocation is a valid/applicable Git form.
8. Forge client leaves that do **not** collide with Git top-level nouns sit **flat beside** Git (`pr`, `issue`, `auth`, `release`, `label`, `run`, `workflow`, `search`, `api`, `org`, …) per [[sot:root/arks-hub-cli-surface]].
9. Nested **`arks hub …`** is **transitional only** (HubAliasLayer forward) — not the taught primary forge noun.
10. **`arks serve`** = GitHub-class host umbrella (HTTP+SSH+forge API); flags `--no-http` / `--no-ssh` / `--no-forge`; nested `serve config`; SSH / forge-in-serve may be Missing / fail-closed ([[sot:root/arks-self-host-server]]).
11. **`arks daemon`** = `git-daemon` symmetry peer — **not** the product forge host.
12. **`arks agent …`** remains optional AgentLayer; not git-alias DoD.

### Overlap leaves — input priority + unified output (arks personality)

Applies to nouns that exist in **both** Git and `gh` inventories under the native `arks` argv space (at least: **`status`**, **`config`**, **`repo`**, and any future Registry-marked overlap).

13. **Do not hard-fork** overlapping leaves into separate primary top-level names. Keeping `inbox` / `forge-config` (or similar) is **demoted to optional aliases / discoverability synonyms only** if retained — **not** the primary story. This **supersedes** earlier hard-rename advice in [[sot:root/arks-hub-product-intent]] / [[sot:root/arks-hub-cli-surface]] that made `inbox` / `forge-config` the primary forge nouns for `status` / `config`.
14. **Input / argv parse priority (arks personality):**
    1. **Rank 0 — Git grammar:** try Git parse/applicability for this invocation first.
    2. **Rank 1 — gh grammar:** if not a valid/applicable Git parse for this invocation, fall through to gh/forge-client grammar.
    3. Exclusive Git-only or exclusive forge-only leaves skip the unused rank.
15. **Output (arks personality, default):** emit a **unified combined view** — Git section + Forge section (or one merged management layout) — **not** “pick one peer’s stdout only.”
16. **Single-peer opt-out flags (design names; Partial until Claimed):** automation MAY request single-peer output/exit via flags such as `--porcelain` (Git script form), `--git-only`, `--forge-only` (exact flag set Partial until clap surface Claimed). When set, output and exit follow that peer only.
17. **`git` / `gh` personalities** stay **peer-pure**: no unified multi-section output requirement; no cross-peer fallthrough unless that peer’s own CLI already behaves that way.

### Exit-code contract = triple personality

18. Exit codes follow the **same triple** as output personality:

| Personality | Exit-code policy |
|-------------|------------------|
| `git` | Stock **git** exit-code semantics for that command (alias drop-in / CI) |
| `gh` | Stock **gh** exit-code semantics for that command (alias drop-in / CI) |
| `arks` | **Unified exit-code policy** (below) |

19. **Unified default (lean fail-closed):** for an overlap (or multi-layer) `arks` invocation that runs more than one layer, exit **non-zero if either layer fails** (logical OR of failure). Success (0) only when every enabled layer that ran succeeded (or was honestly skipped as inapplicable).
20. **Single-peer flags** under `arks` force that peer’s exit semantics only.
21. **Structured multi-status** (e.g. machine-readable per-layer codes) MAY be added later behind an explicit flag; it is **not** the default. Default remains the fail-closed OR policy so scripts that forget flags do not treat a half-failed unified run as success.
22. **Honesty:** exact numeric tables per command are **Partial** until specified + evidenced. This SoT locks the **contract surface** (triple personality + fail-closed unified default) — **do not invent Claimed exit numbers** here. Stable skeleton classes (0 / 1 / 2) and stream rules: [[sot:root/arks-cli-output-exit-contract]].

### CommandLayer + Registry factory (`crates/arks`)

23. Replace the long-term growth of a single giant `match` with a lean **Registry** of **CommandLayer** implementations. Design-only until landed; tip remains Partial.

#### Interfaces (design sketch — not Claimed)

```text
Personality: git | gh | arks

trait CommandLayer {
  fn id(&self) -> LayerId;
  fn owns(&self, personality: Personality, argv: &ArgvView) -> OwnOffer;
  // OwnOffer: Exclusive | OverlapCandidate { rank } | Decline
  fn parse(&self, personality: Personality, argv: &ArgvView) -> ParseResult;
  fn run(&self, ctx: &ExecCtx, plan: Plan) -> LayerOutcome; // status + payload
}

trait OverlapFacade {
  // For Registry-marked overlap leaves under personality=arks:
  // parse(git rank0 → gh rank1) + render(unified) + ExitCodePolicy
  fn dispatch(&self, argv: &ArgvView) -> FacadeOutcome;
}

trait ExitCodePolicy {
  fn map(&self, personality: Personality, outcome: &FacadeOutcome) -> i32;
  // git → git mapper; gh → gh mapper; arks → unified fail-closed OR (default)
}

struct Registry { layers: Vec<Box<dyn CommandLayer>>; facades: … }
```

24. **Layers (registration set):**

| Layer | Role |
|-------|------|
| `GitPorcelainLayer` | Git porcelain / plumbing / credential / lease / lock / … |
| `GhClientLayer` | Flat forge client leaves (`pr`, `issue`, `auth`, …) |
| `ServeLayer` | `serve` / transitional `serve-http` / nested `serve config` |
| `DaemonLayer` | `daemon` (`git://` peer) |
| `AgentLayer` | optional `agent …` |
| `HubAliasLayer` | transitional `hub …` → flat / serve forge module |

25. **Registration order (normative design):**
    1. Resolve **Personality**.
    2. If argv is transitional `hub` → `HubAliasLayer` rewrite, then continue.
    3. Registry lookup by leaf name.
    4. If leaf is **Overlap** and personality is `arks` → **OverlapFacade** (git0 → gh1 parse; unified render; unified ExitCodePolicy).
    5. Else exclusive leaf → owning layer only; ExitCodePolicy for current personality.
    6. Unknown leaf → fail-closed refuse (non-zero).

26. **Registry marks** each top-level leaf as **`Overlap`** or **`Exclusive`** (git-only, forge-only, serve, daemon, agent, …). Collision ownership for overlap leaves is **Facade + rank**, not “rename the noun.”

27. **Crate placement:** factory lives in outer CLI crate `arks`; layers call inward libs (`arks-*`, `arks-hub`, `arks-daemon`, `arks-agent`, …). Libs MUST NOT depend on the CLI bin ([[sot:root/crate-architecture]]).

### Relationship diagram

```text
argv0 / ARKS_PERSONALITY
        │
        ▼
┌───────────────────┐
│ PersonalityResolver│  → git | gh | arks
└─────────┬─────────┘
          ▼
┌───────────────────┐
│ Registry            │  Exclusive vs Overlap marks
└─────────┬─────────┘
          ├─ Exclusive → CommandLayer::parse/run → ExitCodePolicy(personality)
          └─ Overlap + arks → OverlapFacade
                    ├─ parse Git (0) → else gh (1)
                    ├─ render unified (unless --git-only/--forge-only/…)
                    └─ ExitCodePolicy(arks) = fail if either layer fails
```

### Tip honesty (Partial)

| Surface | Tip honesty (design SoT ≠ Claimed) |
|---------|-------------------------------------|
| PersonalityResolver / ARKS_PERSONALITY | **Partial** (skeleton MAY resolve env/argv0; not Claimed drop-in proof) |
| CommandLayer + Registry factory | **Partial** (skeleton Registry wraps existing git match; not a Claimed rewrite) |
| OverlapFacade unified status/config/repo | **Partial** (git path preserved; forge section stub OK) |
| ExitCodePolicy triple | **Partial** (OR-of-failure unit-tested for `arks`; per-cmd peer tables open; **≠ drop-in Claimed**) |
| Flat forge leaves on bare `arks` | **Partial** (forward/stub; nested `hub` transitional; IW-HUB-FLAT) |
| `arks serve --no-forge` / forge-in-serve | **Partial** cohost; hosting e2e Claimed Missing (IW-FORGE-SERVE) |
| Git porcelain under `arks` / alias | Per [[sot:root/cli-surface]] Claimed inventory — unchanged by this SoT |
| `git`/`gh` personality drop-in Claimed | **Missing** until CLI matrix receipt proves peer stdout+exit under shim/`ARKS_PERSONALITY` |

## Why

One binary can honestly serve three muscle memories: stock `git`, stock `gh`, and a human-centered `arks` merge. Keeping overlap nouns shared with **git-then-gh parse** and **unified render/exit** avoids teaching a third vocabulary (`inbox` / `forge-config`) while preserving CI drop-in via peer personalities. A small Registry/Facade/ExitCodePolicy factory keeps ownership explicit without growing an unreviewable `match`.

## Verification

1. `spec check` clean for this SoT.
2. No Claimed language for personality/factory/overlap/exit tables until evidence exists.
3. When implemented: clean-room recipes for `ARKS_PERSONALITY=git` and `=gh` match peer exit/stdout classes on sampled commands; `arks` overlap default fails closed if either layer fails; single-peer flags isolate one peer.
4. Docs/help list personality resolution and state that unified `arks` is not the CI drop-in path unless flags force single-peer.

## Open decisions

| Decision | Status | Notes |
|----------|--------|-------|
| Exact overlap leaf set beyond status/config/repo | **Open** | Registry-owned; amend when matrix proves more collisions |
| Exact clap names for `--git-only` / `--forge-only` / machine multi-status | **Partial** | Design names only until Claimed |
| Whether optional `inbox`/`forge-config` aliases ship | **Open** | Demoted; MAY omit entirely |
| Per-command unified numeric exit table | **Partial** | Default OR-of-failure until per-command rows land |
