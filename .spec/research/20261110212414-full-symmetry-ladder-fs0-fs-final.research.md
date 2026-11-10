---
id: research:root/20261110212414
title: Full Symmetry ladder (FS0–FS_final) — git feature parity DoD
status: active
created: 2026-11-10T21:24:14Z
updated: 2026-11-10T21:24:14Z
tags:
  [
    full-symmetry,
    fs-ladder,
    git-parity,
    human-agent,
    interactive,
    proc-receive,
    lfs,
    bundle-uri,
    replace-refs,
    dod,
  ]
rel:
  depends_on:
    - research:root/20261110105436
    - research:root/20261110205403
    - research:root/20261110211804
    - sot:root/production-residual-register
    - sot:root/product-intent
    - sot:root/cli-surface
    - sot:root/arks-server
    - sot:root/arks-remote
    - sot:root/cli-flag-parity-complete
    - sot:root/public-surface
---

# Full Symmetry ladder (FS0–FS_final)

## Question

Owner correction: product DoD is **complete symmetry with git** — every capability stock git exposes that arks still marks N/A, refuse, defer, or OOS must become **(1) Implemented + Claimed** with Specfold SoT + evidence + clean-room git↔arks symmetry e2e, **or (2) Explicitly impossible** with a written technical/legal reason (not “agent bar” convenience) plus the closest parity slice.

Prior research that left pixel TUI / proc-receive / GUI/LFS unscheduled ([[research:root/20261110211804]] hard-block 3; S4/O3 “agent non-interactive N/A”; product-intent GUI/LFS exclusions as lasting Done) is **overruled for feature-parity scheduling**. L9 crates.io timing and live G8 latency SLO remain **out of this ladder** (distribution / remasure — not git CLI feature symmetry).

## Method

1. Inventory every asymmetric row still labeled N/A / refuse / defer / OOS across `docs/git-vs-arks.md`, residual register §F P4–P5, O3/O4 retain lists, and owning SoTs.
2. Classify each as **implement**, **SoT-amend → implement**, **closest-parity slice**, or **impossible (technical/legal)** — never “agent non-interactive convenience.”
3. Order an **FS0…FS_final** ladder by human+agent git muscle-memory impact → protocol honesty → ecosystem companions → hostile/host-OS edges.
4. Reopen Specfold SoT before Claimed; no invent PASS; English SoT; no OWNER.md; no force-push; Docker-only heavy; halt OOM/stampede.

## DoD (normative)

| Rule | Meaning |
|------|---------|
| **Symmetry** | For every real git feature arks still asymmetrically refuses/N/A/OOS: Claimed with evidence **or** written impossible + closest slice |
| **Human + agent** | Interactive TTY flows are in-scope; scripted env vars are additive, not a substitute Claimed bar that forever excludes TTY |
| **L9** | Distribution policy — **closed**; do not confuse with feature symmetry |
| **G8 live SLO** | Performance remasure — secrets still operator; feature work proceeds without waiting |
| **FS_final** | Symmetry table has **no casual N/A** left for real git features |

## Tip honesty baseline

| Item | Value |
|------|-------|
| Tip (research open) | ≥ `be54977` |
| Prior ladders | L0–L_final Done; S0–S_final Done; O1–O_final Done |
| Overruled | [[research:root/20261110211804]] §Hard block 3 “do not schedule pixel TUI / proc-receive / GUI/LFS” |
| Still closed (non-FS) | **L9**; live G8 PASS until secrets |

---

## FS0 — Full asymmetry inventory

### A. Interactive / patch (formerly “agent N/A”)

| Gap | Git | arks today | FS class | Ladder |
|-----|-----|------------|----------|--------|
| `rebase -i` todo via editor | Opens todo in `GIT_SEQUENCE_EDITOR` → editor fallback | Requires SEQUENCE_EDITOR only; TTY/editor fallback refused | **SoT-amend → implement** | **FS1** |
| `add -p` TTY prompt loop | Terminal y/n/a/d/q (hunk-level) | Scripted/non-TTY only; TTY refuses; file-granularity | **SoT-amend → implement** (TTY file-granularity first; hunk later) | **FS1** then **FS3** |
| Editor `commit` without `-m` | `GIT_EDITOR` / `core.editor` | Scripted editor Claimed (O3) | Already Claimed; keep regression | — |
| `add -i` menu | Interactive command menu | N/A refuse | **Implement** | **FS3** |
| `add -e` hunk edit | Edit patch in editor | N/A refuse | **Implement** | **FS3** |
| `stash -p` / `checkout -p` / `reset -p` | Patch prompts | Unclaimed / refuse | **Implement** | **FS3** |
| Full curses color TUI chrome | Optional git UI polish | Not required for semantic parity | Closest = TTY prompt + editor (not separate curses clone) | **FS1** notes |
| mergetool / difftool **GUI** programs | External tools | N/A | Closest = honor configured tool commands when set; pixel GUI identity not required | **FS7** |

### B. Server / protocol

| Gap | Git | arks today | FS class | Ladder |
|-----|-----|------------|----------|--------|
| `proc-receive` advertise + rewrite | Hook + report-status-v2 option lines | MUST NOT advertise | **SoT-amend → implement** | **FS2** |
| HOOK-008 proc-receive / push-options races | Hostile concurrency edge | Catalog N/A | Hostile harness after FS2 Claimed | **FS8** |
| force-with-lease on dumb WebDAV | Not generally meaningful on dumb | N/A | Closest = keep smart-pack lease Claimed; document dumb non-applicability | **FS2** note |
| Full Apache litmus / every DAV class | Broader than git-http-push | G41 subset Claimed | Expand litmus only if SoTable | **FS5** |
| Extra filter-specs beyond `blob:none`/`tree:0` | Many filter forms | Refuse remainder | **SoT-amend → implement** per-spec | **FS4** |
| Bundle URI clone | `bundle-uri` capability | Catalog NET-006 N/A | **Implement** client/server subset | **FS5** |
| `git replace` / replace refs | First-class | Catalog REF-017 N/A concurrent; CLI may be partial | **Implement** CLI + concurrent honesty | **FS5** |
| Shadow refs (`refs/pull/…`) | Host convention; git stores ordinary refs | Catalog MISC-006 N/A | Closest = ordinary ref read/write under `refs/`; forge UX still out | **FS6** |
| `update-server-info` races | Dumb HTTP helper | Catalog MAINT-004 N/A | **Implement** `update-server-info` + race harness | **FS5** |

### C. Ecosystem companions (git exposes / ships with)

| Gap | Git | arks today | FS class | Ladder |
|-----|-----|------------|----------|--------|
| Git LFS | Separate `git-lfs` + filter process; `git lfs` UX | Product OOS exclusion | Closest = long-running filter process + pointer smudge/clean parity; full LFS server optional | **FS6** |
| `scalar` | Microsoft VFS helper | OOS | Closest = document N/A with reason (not required for git porcelain parity; scalar ≠ core git feature set) **or** minimal enlistment stubs if owner insists | **FS6** note |
| `gitk` / `git-gui` / citool | Tcl/Tk GUIs invoked via git | OOS | **Closest parity**: not embeddable as CLI engine duty; ship refuse + documented external-tool handoff; full GUI = separate product SoT | **FS7** |

### D. Credentials / forge-adjacent

| Gap | Git | arks today | FS class | Ladder |
|-----|-----|------------|----------|--------|
| Credential helper / askpass / TTY / OS wallets | Yes | G33 Claimed | Done | — |
| Forge OAuth device/browser flows | Via helpers / hosts | Refuse | Closest = honor external `credential.helper` that speaks OAuth; in-arks browser OAuth UX = forge product (legal/product boundary) | **FS7** |
| Checks / merge-queue / Issues UX | Host APIs | Out | **Impossible as git CLI** — not a git feature; use `gh`/hosts. Closest = none required for git symmetry | **Impossible** (documented) |

### E. Hostile / host-OS catalog (not casual feature N/A)

| Gap | Class | Ladder |
|-----|-------|--------|
| FS-007 cloud sync on `.git` | Environmental hazard, not a git feature | **Impossible** (document) + closest = refuse/detect when known sync markers |
| FS-014 SELinux/AppArmor deny lock rename | Host policy | **Impossible** to “implement SELinux”; closest = clear errno honesty |
| OBJ-004 SHA-1 collision / sha256 transition hostile | Research/hostile inputs | **FS8** dedicated harness after sha256 product claim (if any) |
| WIN-003 OneDrive placeholders | Host OS | **Impossible** as feature; closest = errno / stub detection honesty |

### F. Explicitly out of *this* ladder (not feature symmetry)

| Item | Reason |
|------|--------|
| **L9** crates.io / docs.rs timing | Distribution policy |
| Live G8 latency SLO / forge throughput PASS | Performance remasure; secrets operator |
| Agent frameworks / IDE products | Consumers of arks, not git features |
| Docker as end-user runtime | Packaging choice |
| CELIR ObjectStore | Refuse-closed alternate store (not a git feature gap) |
| R6 Git pin advance | Process hygiene |

---

## Ordered FS ladder

| # | Goal | Exit criteria | Status |
|---|------|---------------|--------|
| **FS0** | Inventory lock (this note) | Every asymmetric row mapped → implement / closest / impossible; residual + comparison cite this research | **Done** (this note) |
| **FS1** | Interactive TTY + editor fallback | `rebase -i` uses SEQUENCE_EDITOR **or** editor fallback (git var parity); `add -p` TTY prompt loop (file-granularity Claimed); scripted paths retained; SoT/ADR + evidence; no invent curses clone | **Done** ([[adr:root/20261110212430]] + [[evidence:root/20261110212530]]) |
| **FS2** | `proc-receive` | Advertise when hook present; spawn proc-receive; honor rewrite commands; report-status-v2 option lines as claimed; ≠ HOOK-008 race Claimed yet | **Done** ([[adr:root/20261110212500]] + [[evidence:root/20261110212545]]) |
| **FS3** | Patch siblings | `add -i` menu subset; `add -e`; hunk-level `-p`; `stash -p` / related `-p` | **Done** ([[adr:root/20261110213500]] + [[evidence:root/20261110213530]]) |
| **FS4** | Extra filter-specs | `tree:<depth>` + `blob:limit=<n>`; refuse unlisted | **Done** (same ADR/evidence) |
| **FS5** | Bundle URI / replace / update-server-info | `replace` CLI; local `--bundle-uri`; info/refs.lock refuse | **Done** (HTTP bundle race → residual) |
| **FS6** | LFS closest + shadow-ref ordinary refs | `filter.*.clean` honor; scalar refuse+reason; ordinary refs already store `refs/pull` | **Done** (smudge/server residual) |
| **FS7** | GUI/OAuth closest slices | External `gui`/`gitk`/`citool` handoff; OAuth via external helper; forge UX impossible | **Done** |
| **FS8** | Hostile catalog edges | Documented impossibles + HOOK-008/NET-006/OBJ-004 remain reopen harness rows | **Done** (harness residual honest) |
| **FS_final** | Symmetry currency | No *casual* N/A; remaining rows are Claimed, Impossible, or explicit reopen harness | **Done** ([[adr:root/20261110213500]] + [[evidence:root/20261110213530]]; **L9**/G8 non-FS) |

### Non-goals (still)

- Inventing Claimed/PASS without SoT + evidence.
- Reopening **L9** as FS work.
- Waiting on G8 secrets to proceed FS1+.
- Force-push / OWNER.md / host stampede.

## Execution note (this round)

1. Publish **FS0** (this note); supersede hard-block-3 “leave deferred” recommendation for scheduling.
2. ADR + SoT amend for **FS1** (human+agent interactive).
3. Implement FS1 wire; host evidence; update comparison/limits/residual.
4. Proceed toward **FS2** without stopping at research-only.

## Verification

1. Residual register / AGENTS / `docs/git-vs-arks.md` cite this research as the **active** full-symmetry ladder after O_final.
2. FS0 table covers interactive, proc-receive, DAV, filters, LFS, bundle URI, replace, shadow refs, update-server-info, credential/OAuth, GUI, and catalog edges.
3. Each FS# Claimed only with SoT amend + evidence; Docker sequential for heavy gates; halt on OOM/stampede.
4. **FS_final** forbidden while casual N/A remains for real git features.
