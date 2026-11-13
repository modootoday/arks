---
id: sot:root/arks-dod-behavioral
title: Behavioral clean-room DoD (peer action symmetry)
status: active
created: 2026-11-13T10:41:23Z
updated: 2026-11-13T10:54:07Z
tags: [dod, behavioral, verify, clean-room, docker, symmetry, git, daemon, hub, porcelain, no-invent-claimed, strict, met]
verify: manual
rel:
  depends_on:
    - research:root/20261113194123
    - research:root/20261113105000
    - sot:root/arks-dod-optimized
    - sot:root/verify-cleanroom-symmetry-matrix
    - sot:root/production-residual-register
    - sot:root/arks-claimed-residual-gates
    - sot:root/arks-native-e2e-checklist
    - sot:root/arks-cli-personality-factory
    - sot:root/arks-cli-output-exit-contract
    - sot:root/cli-git-help-matrix
    - sot:root/arks-hub-cli-surface
    - sot:root/arks-hub-gh-command-matrix
    - sot:root/arks-daemon-git-command-matrix
    - sot:root/arks-daemon-product-intent
    - sot:root/arks-self-host-server
    - sot:root/product-verification-clean-room
    - sot:root/verify-directory-conventions
    - sot:root/language-english-only
  evidence:
    - evidence:root/20261113105407-dod-behavioral-cli
    - evidence:root/20261113104851-dod-behavioral-cli
    - evidence:root/20261113053511-dod-optimized-cli
    - evidence:root/20261113053516-self-host-forge-e2e
    - evidence:root/20261111050502-hub-stubs-exhaust
  governs:
    - verify/scripts/run-symmetry-dod.sh
    - verify/scripts/run-cli-git-arks-matrix-e2e.sh
    - verify/scripts/run-daemon-git-arks-matrix-e2e.sh
    - verify/scripts/run-hub-gh-arks-matrix-e2e.sh
    - verify/scripts/run-self-host-e2e.sh
    - verify/scripts/run-claimed-residual-dod.sh
---

# Behavioral clean-room DoD (peer action symmetry)

Version **0.1.1**. Normative **Definition of Done** after owner rejection of “CLI exists / Missing=0” as complete. Research: [[research:root/20261113194123]]. Umbrella inventory: [[sot:root/verify-cleanroom-symmetry-matrix]]. Prior optimized bar: [[sot:root/arks-dod-optimized]] — **necessary floors only; insufficient alone**.

**Status: behavioral DoD met** (tip carrying [[evidence:root/20261113105407-dod-behavioral-cli]] + floor OD + hub Refuse×6 + daemon Parity + hub Claimed IW receipts). Remaining surfaces are SoT §B OUT / Refuse terminals only — **not** DoD gaps.

**Product goal (owner):** git ↔ `arks` / git-daemon ↔ `arks daemon` / gh ↔ flat forge client — **behavioral** symmetry (peer-comparable **actions**, exit classes, stdout/stderr contracts). Help-only stubs, clap-accept, and exit-2 for DoD-IN leaves **do not count**. **No invent Claimed/PASS.** Product paths **MUST NOT** spawn `git` / `gh`. Judge = Linux Docker `arks-verify-m1:0.1.0`, pin Git **2.39.5**, `ARKS_DOD_STRICT=1` fail-closed. **No origin push** from this SoT.

## Explicit law (ABSOLUTE)

1. **Inventory Missing=0 alone ≠ complete.**
2. For any DoD-IN command / flag / leaf: **stub**, help-only registration, or **exit-2 / NotImplemented** that does not perform the requested peer work = **DoD gap**.
3. **Claimed** requires Docker STRICT receipt (or cited carrying evidence) showing peer-comparable behavior. Smoke/SKIP ≠ Claimed.
4. Prefer honest **Refuse** (with reason + SoT cite) over silent stubs for IN items that cannot be Claimed.
5. Previous public story “optimized DoD met” / “Missing=0 DoD met” is **superseded** — do not report behavioral DoD met from those alone.

## Rule

### A — Must-pass (behavioral DoD IN)

| ID | Bar | STRICT / compose | Honesty |
|----|-----|------------------|---------|
| **BP-FLOOR** | All [[sot:root/arks-dod-optimized]] §A rows still green (P1–P3 Missing=0 floors, R-HOST, R-B, OD-UNIFIED, OD-SERVE-CFG, OD-COMMIT-HONESTY) | existing `dod-*` + STRICT | **Claimed carrying** [[evidence:root/20261113053511-dod-optimized-cli]] + [[evidence:root/20261113053516-self-host-forge-e2e]] |
| **BP-PORCELAIN** | Named porcelain set (§C) under `ARKS_PERSONALITY=git`: peer-comparable **actions** + exit/stdout **classes** vs pin git **2.39.5** for SoT scenarios | `dod-cli-matrix` + `ARKS_DOD_BEHAVIORAL=1` (or STRICT) | **Claimed** [[evidence:root/20261113105407-dod-behavioral-cli]] — full §C including former open 14 (`show`…`push`); remotes via `arks daemon` git://; stub/exit-2 on §C = **FAIL** |
| **BP-DROPIN** | Personality drop-in: `git` personality peer-pure stdout+exit on §C samples; `gh` personality peer-pure on Claimed hub leaves already in IW drop-in/expand | `dod-cli-matrix` / hub matrix | **Claimed** same receipt (git §C samples + gh `pr --help`); prior IW drop-in/expand/survey remain carrying |
| **BP-DAEMON** | Peer-mapped daemon flags that change wire/archive behavior: living Parity rows stay Parity-carrying; no reintroduction of honest-stub for Parity flags | `dod-daemon-matrix` | Parity×20 carrying; SysV / privilege-drop / inetd Refuse **≠** gap |
| **BP-HUB** | Hub v1 leaves marked Claimed are **real** (REST/metadata paths with receipts). Terminal Partial×6 (§D) either **graduate Claimed** with evidence or **Refuse** with reason — Partial stub **≠** DoD OK | `dod-hub-matrix` | **Claimed×94** via IW-HUB-* receipts ([[evidence:root/20261111050502-hub-stubs-exhaust]] + FLAT/MUTATE); **terminal×6 → Refuse/N/A**; later serve Partial **OUT** |

**PASS** = every IN row green under STRICT with receipt (or carrying evidence cited). **FAIL** = any IN stub/red/Missing under STRICT. **Status now: behavioral DoD met.**

### B — Explicitly OUT of behavioral DoD

| Surface | Count / note | Disposition |
|---------|--------------|-------------|
| Hub `phase=later` Deferred | **×119** | Non-DoD |
| Hub OUT Refuse/N/A | extension/copilot/skill/… | Non-DoD |
| CLI survey Partial outside §C | remainder after BP set | Non-DoD honest Partial; **no stampede** |
| GUI / interactive TUI / mergetool / gitk | Refuse inventory | Non-DoD |
| Classic SysV double-fork; privilege-drop; inetd; documented daemon Refuse | Daemon | Terminal Refuse |
| Impossible / L9 / live G8 latency SLO | Residual next-queue | Non-DoD |
| Agent as fourth symmetry pillar | Future | Not DoD until own SoT+gate |

### C — BP-PORCELAIN named set (bounds)

Derived from official **giteveryday** (~20 everyday cmds) + owner remotes list + pin **2.39.5** contracts ([[research:root/20261113194123]]). **Not** the full pin `list-cmds` / Partial×117.

| Bucket | Commands (DoD-IN) |
|--------|-------------------|
| Local core | `init`, `status`, `add`, `commit`, `log`, `show`, `diff`, `branch`, `checkout`, `switch`, `restore`, `reset`, `rm`, `mv`, `merge`, `rebase`, `tag`, `stash` |
| Remotes | `clone`, `fetch`, `pull`, `push`, `remote` with subcommands **list** (bare), **add**, **remove\|rm**, **show**, **get-url**, **set-url** |

#### Acceptance (each §C leaf)

1. Under `ARKS_PERSONALITY=git`, a Docker STRICT recipe exercises a **core workflow** scenario (not only `--help`).
2. Exit code **class** matches pin git for that scenario ([[sot:root/arks-cli-output-exit-contract]]).
3. Stdout carries the peer **payload class** (e.g. `status --porcelain` script form; `log --oneline` one line; `remote` prints names); stderr for diagnostics/refuse.
4. **Refuse as Claimed:** help-only; clap-accept without body; exit-2 / `NotImplemented` for an in-scope subcommand; inventing PASS from Missing=0 CSV alone.
5. Flag depth beyond the core scenario MAY remain Partial without failing BP-PORCELAIN **if** the core scenario is Claimed and exotic flags are honest Partial/Refuse (not silent success).

### D — Hub terminal Partial×6 (must resolve)

| Leaf | Prefer | If cannot Claim |
|------|--------|-----------------|
| `auth refresh` | Graduate with OAuth-local evidence | **Refuse** — external OAuth refresh out of local-forge v1 |
| `pr diff` | Graduate compare/objects without spawning git | **Refuse** — requires git-object compare beyond hub metadata |
| `pr checks` | Graduate local check-run metadata | **Refuse** — live CI check-runs out of scope |
| `pr update-branch` | Graduate metadata-only | **Refuse** — requires worktree/git merge |
| `pr revert` | Graduate metadata-only | **Refuse** — requires git revert object write |
| `repo fork` | Graduate forge-metadata fork | **Refuse** — object copy / git spawn forbidden |

CSV `status_now` MUST be **Claimed** or **Refuse/N/A** for these six before behavioral DoD met — **Partial stub is not terminal OK**.

### E — Relationship to prior SoTs

1. [[sot:root/arks-dod-optimized]] remains the **floor** (Missing=0 + OD-* + R-*). Public “DoD met” for **behavioral** symmetry cites **this** SoT.
2. [[sot:root/verify-cleanroom-symmetry-matrix]] tip currency MUST cite behavioral bar; prior “optimized DoD met” language is superseded.
3. [[sot:root/production-residual-register]] MUST record behavioral DoD open/met honestly.
4. Carrying personality/hub/daemon Claimed evidence remains valid **where it proves §A rows** — not a license to invent BP-PORCELAIN Claimed from survey exit-class rows alone.

### F — Completion protocol (Phase B)

For each IN gap only, in SoT order (**BP-PORCELAIN** remotes stubs → local core peer-compare → BP-DROPIN → BP-DAEMON honesty → BP-HUB terminal resolve → BP-FLOOR reconfirm):

1. Spec already names the bar (this SoT) — then minimal product/harness.
2. Docker STRICT green (`ARKS_DOD_BEHAVIORAL=1` and/or `ARKS_DOD_STRICT=1`).
3. `.spec/evidence/` receipt **or** honest Refuse reclassification (amend CSV + cite — never invent Claimed).

## Why

Missing=0 invited stub-complete storytelling. Owner requires peer-comparable **behavior**. A named everyday porcelain set + daemon Parity honesty + hub Claimed-or-Refuse restores that bar without Partial×117 stampede.

## Verification

1. `spec check` clean for this SoT + research link.
2. IN table rows either cite carrying tip evidence or new BP-* receipts under STRICT.
3. OUT table / §C bounds are never treated as “implement everything Partial.”
4. Public docs MAY say “behavioral DoD met” only when §A is green — **no invent PASS**.
