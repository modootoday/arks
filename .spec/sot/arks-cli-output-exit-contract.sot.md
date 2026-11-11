---
id: sot:root/arks-cli-output-exit-contract
title: arks CLI output formats and exit-code contracts
status: active
created: 2026-11-10T20:20:00Z
updated: 2026-11-10T20:20:00Z
tags: [cli, stdout, stderr, porcelain, machine, exit-code, personality, arks, git, gh, overlap, stub, design, partial]
verify: manual
rel:
  depends_on:
    - sot:root/arks-cli-personality-factory
    - sot:root/cli-surface
    - sot:root/arks-hub-cli-surface
    - sot:root/arks-hub-product-intent
    - sot:root/arks-self-host-server
    - sot:root/arks-daemon-product-intent
    - sot:root/arks-agent-cli-surface
    - sot:root/product-naming
    - sot:root/public-surface
  evidence: []
---

# arks CLI output formats and exit-code contracts

Document revision **0.1.0** (SoT edit counter only — **not** a product/crate release). Normative **contract surface** for **stdout / stderr / machine flags** and **exit codes** under the locked triple personality (`arks` | `git` | `gh`). Dispatch / factory ownership: [[sot:root/arks-cli-personality-factory]]. Command inventory: [[sot:root/cli-surface]] (Git), [[sot:root/arks-hub-cli-surface]] (forge), [[sot:root/arks-self-host-server]] (serve), [[sot:root/arks-daemon-product-intent]] (daemon), [[sot:root/arks-agent-cli-surface]] (agent).

**Honesty:** This SoT locks **shapes and classes**. Exact per-command numeric tables beyond the class tables below remain **Partial** until evidenced. **Do not invent Claimed** exit numbers or byte-identical peer stderr from this document alone. Document revision ≠ crates.io/`arks` version bump.

## Rule

### Streams (all personalities)

1. **stdout** — primary successful payload (human or machine). Helpers that print usage on `--help` MAY use stdout (Git-like) or stderr; once a leaf Claimed, keep that leaf consistent.
2. **stderr** — diagnostics, warnings, refuse/Partial messages, progress that must not pollute machine stdout.
3. **Never** exit **0** after a refuse / unimplemented / Partial stub that did not complete the requested work.
4. Exact **byte identity** with stock `git` / `gh` stderr is **not** required unless a clean-room gate asserts it ([[sot:root/cli-surface]]). Behavioral class (success vs failure; porcelain vs human) **is** required for Claimed rows.

### Machine / porcelain / single-peer flags (design names; Partial until clap Claimed)

| Flag (design) | Intent | Output | Exit |
|---------------|--------|--------|------|
| `--porcelain` | Git script form on overlap/git leaves | Git porcelain only | Git peer |
| `--git-only` | Force git peer under `arks` | Git section only | Git peer |
| `--forge-only` | Force forge peer under `arks` | Forge section only | gh-class peer |
| (future) multi-status machine flag | Per-layer codes | Structured stdout | **Not** default; default stays fail-closed OR |

5. Under personalities **`git`** / **`gh`**, single-peer flags are redundant: output and exit are already peer-pure.
6. Under **`arks`**, default overlap output is **unified** (Git section + Forge section, or one merged management layout) unless a single-peer flag is set ([[sot:root/arks-cli-personality-factory]] §Overlap).

### Personality → output shape

| Personality | Output rule |
|-------------|-------------|
| `git` | **Peer-native Git** stdout/stderr shape. No forge unified sections. No requirement to emit forge client output. |
| `gh` | **Peer-native gh** stdout/stderr shape for surveyed forge leaves. No git porcelain unless that `gh` leaf already owns the noun. |
| `arks` | **Git-first merged surface.** Exclusive git / exclusive forge / serve / daemon / agent as registered. **Overlap** leaves (`status`, `config`, `repo`, …): **unified** combined view by default. |

### Personality → exit-code policy (triple)

| Personality | Policy id | Rule |
|-------------|-----------|------|
| `git` | `ExitGitFaithful` | Stock **git** success/failure **classes** for that command (alias / CI drop-in). Undocumented → non-zero on any failure. |
| `gh` | `ExitGhFaithful` | Stock **gh** success/failure **classes** for that command (alias / CI drop-in). Prefer gh-like usage vs runtime distinction when Claimed. |
| `arks` | `ExitArksUnified` | **Fail-closed OR:** if more than one enabled layer ran, exit non-zero if **either** failed; **0** only when every enabled layer that ran succeeded (or was honestly skipped as inapplicable). Single-peer flags → that peer’s mapper only. |

7. **CI guidance:** scripts that today call `git` / `gh` SHOULD use personality `git` / `gh` (alias, shim, or `ARKS_PERSONALITY`) for peer-native stdout **and** peer-native exit. Bare `arks` unified ≠ CI drop-in unless single-peer flags force one peer.

### Stable skeleton exit classes (implementation contract)

Until per-command numeric rows are Claimed, the CLI skeleton MUST use these **stable classes** (not silent success):

| Class | Code | When |
|-------|------|------|
| Success | `0` | Requested work completed (or honest no-op success such as help that lists surface) |
| Generic failure | `1` | Runtime / repo / wire failure on an implemented path; git-faithful default failure |
| Usage / Partial stub / not implemented | `2` | Bad argv; unknown personality; **unimplemented leaf stub**; Partial discoverability refuse; serve module Missing while enabled |
| (Reserved) git special | peer-documented | Only when a Claimed git leaf documents a distinct code (e.g. `diff` 1 = differences) — then `ExitGitFaithful` MUST honor that class |

8. **Stub message shape (stderr):** include the leaf name, state **Partial / not Claimed**, and a SoT cite when practical (e.g. `[[sot:root/arks-hub-cli-surface]]`). Exit **2** (not 0).
9. **Help honesty:** top-level and group help MAY list stub leaves but MUST mark them **Partial/stub** — do not imply Claimed depth.

### Overlap leaves (`status` / `config` / `repo`)

10. **Input (arks):** Git grammar rank 0 → else gh/forge rank 1 ([[sot:root/arks-cli-personality-factory]]).
11. **Output (arks default):** unified Git + Forge sections. **Skeleton honesty:** until the forge overlap facet is Claimed, the Forge section MAY be **honestly skipped** (stderr note) so Claimed git exits are not regressively fail-closed by a Partial stub. `--forge-only` / personality `gh` still route the forge facet (hub stub → non-zero until Claimed).
12. **Exit (arks default):** `ExitArksUnified` — if Git fails → non-zero; if an **enabled** Forge layer runs and fails → non-zero; skipped-as-inapplicable layers do not veto success.
13. **`git` personality:** Git output + `ExitGitFaithful` only (no forge section).
14. **`gh` personality:** Forge/gh grammar + `ExitGhFaithful` only (no git porcelain requirement).

### Serve / daemon / agent / hub stubs

| Surface | Enabled-but-Missing behavior | Exit class |
|---------|------------------------------|------------|
| `arks serve` with SSH and/or forge enabled while Missing | Fail-closed explicit stderr; **no** silent degrade | `2` |
| `arks serve --no-http --no-ssh --no-forge` (nothing left) | Usage error | `2` |
| `arks serve-http` | Transitional ≡ `serve --no-ssh --no-forge` (HTTP git only) | same as HTTP serve path |
| `arks daemon` | Existing daemon path or Partial refuse | non-zero on refuse |
| `arks agent …` | Stub / Partial until Claimed | `2` on unimplemented |
| Flat forge leaves / transitional `hub` | Forward to hub lib **or** stub | hub codes today; stubs → `2` |
| Unknown leaf | Refuse | `2` |

15. **`--no-forge`** is part of the clap surface even while forge-in-serve is Missing: default still **enables** forge intent and fail-closes until Claimed ([[sot:root/arks-self-host-server]]).

### Relationship to factory

```text
PersonalityResolver → Registry / OverlapFacade → layer outcomes
        → ExitCodePolicy.map(personality, outcome) → process exit
```

16. Factory types and registration order: [[sot:root/arks-cli-personality-factory]]. This SoT owns **what** stdout/stderr/exit mean once outcomes exist.

## Why

CI drop-in needs peer-faithful streams and codes; humans need a unified `arks` view without a third vocabulary. Locking classes early lets the CLI skeleton ship stubs with **stable non-zero exits** while deep forge/SSH bodies land later — without inventing Claimed parity.

## Verification

1. `spec check` clean for this SoT.
2. No Claimed language for per-command numeric tables beyond the class table until evidence exists.
3. Skeleton smoke: unknown `ARKS_PERSONALITY` → exit 2; unimplemented flat leaf → exit 2 + Partial stderr; `ARKS_PERSONALITY=git` status uses git path; serve with forge/SSH enabled while Missing → exit 2.
4. Cross-links from [[sot:root/arks-cli-personality-factory]], [[sot:root/cli-surface]], `.spec/README.md`.

## Open decisions

| Decision | Status | Notes |
|----------|--------|-------|
| Exact clap spelling for machine multi-status | **Partial** | Not default |
| Per-command unified numeric rows | **Partial** beyond named Claimed | Named IW-PERSONALITY-EXIT rows Claimed ([[evidence:root/20261111025151-personality-exit-tables]]); OR-of-failure elsewhere |
| Whether help for stubs uses stdout or stderr | **Open** | Prefer consistency per leaf once Claimed |
