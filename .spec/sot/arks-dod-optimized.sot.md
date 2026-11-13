---
id: sot:root/arks-dod-optimized
title: Optimized clean-room DoD (symmetry + related serve)
status: active
created: 2026-11-13T05:30:29Z
updated: 2026-11-13T10:41:23Z
tags: [dod, optimize, verify, clean-room, docker, symmetry, git, daemon, hub, serve, no-invent-claimed, strict, superseded-as-complete]
verify: manual
rel:
  depends_on:
    - research:root/20261113143029
    - sot:root/verify-cleanroom-symmetry-matrix
    - sot:root/production-residual-register
    - sot:root/arks-claimed-residual-gates
    - sot:root/arks-native-e2e-checklist
    - sot:root/arks-cli-personality-factory
    - sot:root/arks-cli-output-exit-contract
    - sot:root/arks-hub-cli-surface
    - sot:root/arks-hub-gh-command-matrix
    - sot:root/arks-daemon-git-command-matrix
    - sot:root/arks-daemon-product-intent
    - sot:root/arks-self-host-server
    - sot:root/product-verification-clean-room
    - sot:root/verify-directory-conventions
    - sot:root/language-english-only
  related:
    - sot:root/arks-dod-behavioral
    - research:root/20261113194123
  evidence:
    - evidence:root/20261113053511-dod-optimized-cli
    - evidence:root/20261113053516-self-host-forge-e2e
  governs:
    - verify/scripts/run-symmetry-dod.sh
    - verify/scripts/run-cli-git-arks-matrix-e2e.sh
    - verify/scripts/run-self-host-e2e.sh
    - verify/scripts/run-claimed-residual-dod.sh
---

# Optimized clean-room DoD (symmetry + related serve)

Version **0.1.2**. **Floor only** after owner correction: Missing=0 + OD-* is **necessary but insufficient**. Normative complete bar is [[sot:root/arks-dod-behavioral]] ([[research:root/20261113194123]]) — peer **behavioral** symmetry; stubs do not count. Research (optimize cut): [[research:root/20261113143029]]. Umbrella inventory: [[sot:root/verify-cleanroom-symmetry-matrix]]. **Tip ≥ Spec `e90d782` + harness receipts:** OD-UNIFIED + OD-COMMIT-HONESTY [[evidence:root/20261113053511-dod-optimized-cli]]; OD-SERVE-CFG (+ IW-FORGE-SERVE) [[evidence:root/20261113053516-self-host-forge-e2e]]; pillars STRICT Missing=0 reconfirmed — **do not report “DoD met” from this SoT alone**.

**Product goal (owner):** git ↔ `arks` / git-daemon ↔ `arks daemon` / gh ↔ flat forge client symmetry — **not** exhausting every Partial to Claimed. **No invent Claimed/PASS.** Product paths **MUST NOT** spawn `git` / `gh`. Judge = Linux Docker `arks-verify-m1:0.1.0`, pin Git **2.39.5**, `ARKS_DOD_STRICT=1` fail-closed. **No origin push** from this SoT.

## Rule

### A — Must-pass (renewed DoD IN)

| ID | Bar | STRICT / compose | Honesty |
|----|-----|------------------|---------|
| **P1-CLI** | `cli-git-help-matrix.csv` **Missing = 0**; sampled dispatch honest | `dod-cli-matrix` / `dod-symmetry` | Flag-depth Partial remainder **≠** this bar |
| **P2-DAEMON** | `daemon-git-arks-e2e-matrix.csv` **Missing = 0**; peer-mapped Parity carrying | `dod-daemon-matrix` | SysV / privilege-drop Refuse **≠** gap |
| **P3-HUB** | Hub v1 **Missing = 0**; Claimed inventory green under existing IW arms | `dod-hub-matrix` | Terminal Partial×6 **≠** Missing; later/OUT **≠** gap |
| **R-HOST** | Related self-host forge hosting e2e Claimed (IW-FORGE-SERVE) | `dod-self-host` | WAN secure-by-default forbidden |
| **R-B** | Claimed residual **B1/B2/B3/B4/B5/B5+/B7(HTTP+SSH+forge)/B9** | `dod-claimed-residual` | Not a fourth symmetry pillar |
| **OD-UNIFIED** | Bounded unified `arks` overlap STRICT arm (below) | `dod-cli-matrix` + `ARKS_DOD_UNIFIED=1` (or STRICT) | **Claimed** [[evidence:root/20261113053511-dod-optimized-cli]]; full forge unified depth still Partial |
| **OD-SERVE-CFG** | `arks serve config` smoke Claimed (below) | `dod-self-host` + `ARKS_DOD_SERVE_CFG=1` (or STRICT) | **Claimed** [[evidence:root/20261113053516-self-host-forge-e2e]]; schema completeness still Partial |
| **OD-COMMIT-HONESTY** | Commit editor / refuse honesty STRICT (below) | `dod-cli-matrix` + `ARKS_DOD_COMMIT_HONESTY=1` (or STRICT) | **Claimed** [[evidence:root/20261113053511-dod-optimized-cli]]; interactive TUI still deferred |

**PASS** = every IN row green under STRICT with receipt (or already carrying evidence cited). **FAIL** = any IN Missing/red under STRICT. Smoke/SKIP without STRICT **MUST NOT** invent PASS.

### B — Explicitly OUT of renewed DoD (non-DoD)

| Surface | Count / note | Disposition |
|---------|--------------|-------------|
| Hub `phase=later` Deferred | **×119** | Non-DoD — do not implement-to-Claim for DoD |
| Hub OUT Refuse/N/A | extension/copilot/skill/… | Non-DoD |
| CLI survey Partial remainder | **×117** after named/survey Claimed subsets | Non-DoD honest Partial; **no stampede** |
| Hub terminal Partial | **×6** — `auth refresh`; `pr diff`; `pr checks`; `pr update-branch`; `pr revert`; `repo fork` | **Keep terminal** — OAuth/CI/git-object; invent forbidden |
| Unified personality overlap beyond OD-UNIFIED | Forge section body depth | Non-DoD Partial |
| Classic SysV double-fork detach | Daemon | Terminal Refuse (peer single-fork+`setsid`) |
| Impossible / L9 / live G8 latency SLO | Residual next-queue | Non-DoD |
| Agent as fourth symmetry pillar | Future | Not DoD until own SoT+gate |

### C — Bounded raises (acceptance)

#### OD-UNIFIED — unified arks-mode STRICT arm

1. Default personality `arks`, overlap leaf `status` (clean repo): exit **0**; stdout contains Git section marker; stderr (or stdout note) records Forge unified section **honest skip / Partial** (no invent forge Claimed).
2. Same leaf with `--git-only`: exit **0**; **no** Forge section requirement.
3. `--porcelain` on overlap selects git-only script form (mode flag accepted).
4. `--forge-only status` (or documented forge facet argv) routes hub facet without spawning `gh`.
5. **Refuse as Claimed:** byte-identical multi-section forge inbox; inventing forge Claimed from skip note alone.

#### OD-SERVE-CFG — serve config smoke

1. `arks serve config path` → exit 0, prints a path.
2. `arks serve config set <allowed-key> <value>` → exit 0 (Partial label OK on stderr/stdout).
3. `arks serve config show` → exit 0 after set.
4. `arks serve config validate` → exit 0 on valid Partial schema (or empty-file honesty).
5. **Refuse as Claimed:** full ops schema / WAN TLS completeness; conflation with `arks config` or hub client config.

#### OD-COMMIT-HONESTY — commit Spec honesty

1. In a dirty or staged repo: `arks commit` with **no** `-m`/`-F`/`-t` and **unset** `GIT_EDITOR`/`ARKS_EDITOR`/`EDITOR` → **non-zero** with honesty message (O3).
2. With scripted editor env writing a message → exit **0** and creates a commit (or equivalent FS1/O3 success).
3. **Refuse as Claimed:** interactive TUI / full editor UX; inventing PASS without Docker recipe.

### D — Relationship to prior SoTs

1. This SoT **narrows** what agents must close for “DoD met.” It does **not** reopen closed Claimed waves or invent new pillars.
2. [[sot:root/verify-cleanroom-symmetry-matrix]] remains the three-pillar umbrella; tip currency MUST cite this optimized bar.
3. [[sot:root/arks-native-e2e-checklist]] MAY list OD-* arms; Partial/OUT rows stay SKIP.
4. Carrying personality/hub Claimed subsets (drop-in, exit tables, survey2/3/4, hub IW-*) remain evidence — **not** a license to stampede Partial×117.

### E — Completion protocol (Phase B)

For each IN gap only:

1. Spec already names the bar (this SoT) — then minimal product/harness.
2. Docker STRICT green.
3. `.spec/evidence/` receipt **or** honest refuse to keep the bar out of DoD (amend this SoT — never invent Claimed).

## Why

Old Missing=0 DoD invited infinite Partial deepening. A renewed bar keeps symmetry honest, cuts thrash surfaces, and raises only three evidence-feasible product smokes operators already need.

## Verification

1. `spec check` clean for this SoT + research link.
2. IN table rows either cite carrying tip evidence or new OD-* receipts under STRICT.
3. OUT table is never treated as must-pass by agents.
4. Public docs MUST NOT say product DoD met from this SoT alone — cite [[sot:root/arks-dod-behavioral]] for behavioral complete; this SoT’s §A green is the **floor** only — **no invent PASS**.
