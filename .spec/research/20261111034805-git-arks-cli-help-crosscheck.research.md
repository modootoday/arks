---
id: research:root/20261111034805
title: Git ↔ arks CLI --help cross-check (command-level)
status: active
created: 2026-11-10T18:48:05Z
updated: 2026-11-10T18:48:05Z
tags: [cli, help, parity, alias, git, arks, inventory, crosscheck, no-invent-claimed]
verify: manual
rel:
  depends_on:
    - sot:root/cli-surface
    - sot:root/cli-flag-parity-complete
    - sot:root/cli-flag-parity-full
    - sot:root/cli-flag-parity-m9
    - sot:root/clean-room-git-pin-advance
    - sot:root/product-verification-clean-room
    - sot:root/production-residual-register
    - research:root/20261108183800
  evidence:
    - evidence:root/20261111034600
---

# Git ↔ arks CLI `--help` cross-check (command-level)

Tip surveyed: **`679a202`** (canon `C:\workspaces\gir`). **No invent Claimed** — this note records live `--help` / dispatch / pin inventory evidence only. Gate re-stamps (G14/G20/G21/…) are **not** re-run here (**no G19**).

## Method

1. **Pin oracle:** Docker `arks-verify-m1:0.1.0` → stock **`git version 2.39.5`** (`verify/images.lock.json`). Enumerate `git --list-cmds=main,others,alias` + `git help -a` sections.
2. **Host skew:** Host `git version 2.54.0.windows.1` listed for delta only — **not** the parity judge.
3. **arks surface:** `target/debug/arks.exe --help` + `crates/arks/src/main.rs` `HELP` + `match cmd.as_str()` arms; spot-check high-traffic flags.
4. **SoT cross-read:** [[sot:root/cli-surface]], [[sot:root/cli-flag-parity-complete]] (active M16), historical [[sot:root/cli-flag-parity-full]] / m9, public `docs/git-vs-arks.md`.
5. **Classification (this survey):**

| Status | Meaning in this note |
|--------|----------------------|
| **Claimed/parity** | **Not assigned.** Survey refuses to invent Claimed without carrying gate re-stamp on this tip. |
| **Parity-candidate** | Present in `arks --help` **and** dispatch; no material SoT command-local N/A called out in the complete ledger for that row — still **not** Claimed here. |
| **Partial** | Dispatch and/or HELP present, but SoT documents command-local N/A / known flag refuse / help-honesty gap / subset claim. |
| **Refuse/N/A** | Explicit refuse arm, SoT product exclusion, GUI/mergetool/instaweb, or git internal `--*` helper. |
| **Missing** | Pin `list-cmds` entry with **no** arks HELP/dispatch (runtime: generic `command not implemented`). |
| **arks-only** | Beyond stock git builtins (product extensions). |

Evidence pack: `.spec/evidence/20261111034600-git-arks-cli-help-crosscheck/` (CSV + spot-check + pin lists).

## Counts (pin 2.39.5 `list-cmds`, n=166)

| Status | Count | Notes |
|--------|------:|-------|
| Claimed/parity (this survey) | **0** | Honesty: do not stamp Claimed from HELP alone |
| Parity-candidate | **21** | HELP + dispatch; no ledger N/A called out here |
| Partial | **60** | Subset / SoT N/A column / help gap / option depth |
| Refuse/N/A | **15** | Internals + GUI tools + `filter-branch` exclude |
| Missing | **70** | Pin inventory absent from arks surface |
| **Sum** | **166** | |

Additional (not in pin `list-cmds`):

| Bucket | Count | Examples |
|--------|------:|----------|
| arks-only (beyond git builtins) | **5** | `forge`, `host-env`, `lease`, `lock`, `serve-http` |
| Companion / refuse arms not in pin list | **6** | `lfs` (HELP); `scalar`/`gitk`/`gui`/`citool`/`backfill` refuse |
| First-party credential helpers (pin may omit OS names) | **3** | `credential-wincred`, `credential-libsecret`, `credential-osxkeychain` |
| Host 2.54-only vs pin | **22+** | e.g. `history`, `refs`, `replay`, `repo`, `send-email` (Windows packaging), `update-git-for-windows`, … |
| `celir` / `celir-*` | **0 shipped** | Runtime `command not implemented` (CELIR admin cancelled / non-product per cli-surface) |

arks dispatch arms ≈ **96**; HELP-listed ≈ **88**. Help honesty gaps (dispatch yes / HELP no): `checkout-index`, `verify-tag`, `help` (meta), plus refuse-only arms.

## Tension vs SoT narrative (important)

[[sot:root/cli-flag-parity-complete]] / [[sot:root/cli-surface]] **M16** bar: every pin inventoriable command/option is **implement** or **N/A exclusion** — no production refuse leftovers.

**Live binary surface disagrees at command level:** **70** pin `list-cmds` names fall through to generic `command not implemented` (Missing). That is **not** the same as an explicit SoT N/A row or dedicated refuse message.

Do **not** treat public `docs/git-vs-arks.md` “Full CLI inventory + flag symmetry Claimed (G20/G21)” as a re-proof of this tip’s HELP completeness. Historical receipts exist; **this survey** only asserts the HELP/dispatch matrix above.

## Missing (70) — pin present, arks absent

### High-alias / user-visible gaps (non-exhaustive priority)

| Command | Runtime | Alias impact |
|---------|---------|--------------|
| `apply` | `command not implemented` | Patch apply workflows |
| `ls-remote` | same | Remote discovery scripts |
| `prune` / `prune-packed` | same | Maintenance muscle memory |
| `cherry` | same | Upstream cherry listing |
| `show-branch` | same | Multi-branch view |
| `rerere` | same | Conflict memory |
| `fast-export` / `fast-import` | same | History interchange |
| `subtree` | same | Vendor subtree flows |
| `request-pull` | same | Patch-request helper |
| `hook` | same | Hook runner CLI |
| `for-each-repo` | same | Multi-repo maintenance |
| `bugreport` / `diagnose` | same | Support packs |
| `diff-files` / `diff-index` | same | Plumbing diffs |
| `fetch-pack` / `send-pack` | same | Low-level pack client |
| `merge-file` / `merge-tree` / `merge-index` | same | Merge plumbing |
| `mktag` / `mktree` | same | Object construction |
| `unpack-objects` / `verify-pack` | same | Pack tooling |
| `stripspace` / `mailinfo` / `mailsplit` | same | `am` ecosystem helpers |
| `credential-cache` / `credential-store` | same | Stock helper names (arks has fill + OS wallets) |
| `multi-pack-index` / `commit-graph` | same | Maint graph/MPI |
| `check-attr` / `check-mailmap` / `check-ref-format` | same | Check plumbing |
| `var` / `column` / `patch-id` / `fmt-merge-msg` / `interpret-trailers` | same | Ancillary |
| `http-fetch` / `http-push` / `upload-archive` | same | Legacy HTTP / archive serve |
| `imap-send` / `quiltimport` / `shell` / `pickaxe` / `get-tar-commit-id` / `show-index` / `pack-redundant` / `unpack-file` | same | Niche / legacy |

### Synonyms (4) — Missing name, covered under another arks cmd

| Pin name | Covered by |
|----------|------------|
| `stage` | `add` |
| `init-db` | `init` |
| `fsck-objects` | `fsck` |
| `whatchanged` | `log` (deprecated synonym) |

### Merge strategy backends (7) — Missing as named cmds

`merge-octopus`, `merge-ours`, `merge-recursive`, `merge-recursive-ours`, `merge-recursive-theirs`, `merge-resolve`, `merge-subtree` — stock git exposes these as `git-merge-*` helpers; arks `merge` refuses `-s octopus` etc. (spot-check).

### Remote helper binaries (6) — Missing

`remote-ext`, `remote-fd`, `remote-ftp`, `remote-ftps`, `remote-http`, `remote-https` — typically invoked via URL scheme, not muscle-memory alias; still absent from arks dispatch.

## Partial (60) — present but not full man-page / SoT N/A columns

Representative SoT / spot-check flag gaps ([[sot:root/cli-flag-parity-complete]] + runtime):

| Command | Evidence |
|---------|----------|
| `status -b` | Spot-check **exit 0** (Sy-1 form works) — still Partial for porcelain v2 extras N/A |
| `log --graph` | Spot-check **exit 0** — `--pretty=format:` full custom still N/A |
| `branch -a` | Spot-check **exit 0** — `--format` full N/A |
| `add --chmod=+x` | Spot-check **exit 0** — mergetool GUI N/A |
| `gc --prune=now` | Spot-check **refuse** CORR-001 (honest) |
| `grep -P` | Spot-check **refuse** (`not claimed`) |
| `diff --word-diff` | Spot-check **unsupported option** |
| `merge -s octopus` | Spot-check **unsupported option** |
| `worktree list --porcelain` | Spot-check **exit 0** — main-worktree remove still N/A |
| Server cmds (`daemon`, `upload-pack`, `receive-pack`, `http-backend`, `update-server-info`) | **KEEP in-scope** per product intent; HELP/dispatch present; full man-page option depth **not** stamped in this survey → Partial |
| `lfs` | Not a pin builtin; HELP + handoff/subset → Partial / companion |

Full Partial list: see CSV `status=Partial`.

## Refuse/N/A (15 on pin list + extra refuse arms)

**On pin matrix:** `filter-branch` (explicit exclude); `difftool` / `difftool--helper` / `mergetool` / `instaweb`; internal `--*` helpers (`add--interactive`, `bisect--helper`, `checkout--worker`, `credential-cache--daemon`, `env--helper`, `fsmonitor--daemon`, `sh-i18n--envsubst`, `submodule--helper`, `upload-archive--writer`, `web--browse`).

**Extra refuse arms (often not in Debian pin `list-cmds`):** `scalar`, `gitk`, `gui`, `citool`, `backfill` — dedicated messages (FS6/FS7 / M15 exclude).

## Parity-candidate (21) — HELP + dispatch; **not Claimed here**

`check-ignore`, `checkout`, `commit`, `commit-tree`, `count-objects`, `ls-tree`, `mv`, `name-rev`, `pack-refs`, `reflog`, `remote`, `restore`, `rm`, `shortlog`, `show-ref`, `switch`, `symbolic-ref`, `update-ref`, `verify-commit`, `version`, `write-tree`

These are the **least contested** command-level rows in this pass. Promoting any to **Claimed** requires citing carrying evidence / gate on **this tip**, not this HELP matrix alone.

## arks-only (beyond git alias)

| Command | Role |
|---------|------|
| `forge` | Thin wrap → host `gh` (Claimed-closest narrative elsewhere; forge host UX non-goal) |
| `host-env` | Cloud-sync / CfAPI / LSM detect + opt-in refuse (FS-007 / WIN-003 / FS-014) |
| `lease` | Product durable leases (M13) — not git man-page inventory |
| `lock` | Inspect/clear stale `*.lock` (M12) |
| `serve-http` | Loopback smart(+dumb) HTTP listener (Wave Sv) — git has `http-backend`/`daemon`, not this name |

`celir-*`: **not implemented** (cancelled / non-product admin UX per cli-surface exclusions).

## Server commands (KEEP)

Per owner intent: `arks daemon`, `upload-pack`, `receive-pack`, `http-backend`, `update-server-info`, `serve-http` remain **in-scope**. They appear on HELP/dispatch. Classified **Partial** here only because this survey did not flag-audit full man pages — **not** because they are out of product scope.

## Host git 2.54 skew (informational)

Host-only names (not in pin 2.39.5 `list-cmds`) include Windows/Git-for-Windows and newer suite entries: `history`, `refs`, `replay`, `repo`, `survey`, `last-modified`, `diff-pairs`, `credential-manager`, `update-git-for-windows`, packaged `lfs`/`gui`/`citool`, etc. Parity is judged against **pin 2.39.5**, not host 2.54.

Debian pin image also omits some upstream names from `list-cmds` (e.g. `send-email` / `gitk` often separate packages) even when `git help -a` text mentions related topics — packaging ≠ full upstream tarball inventory.

## Spot-check summary

File: `spotcheck-high-traffic.txt`

- Working: `status -b`, `log --graph`, `branch -a`, `add --chmod=+x`, `worktree list --porcelain`
- Honest refuse: `gc --prune=now`, `grep -P`, `diff --word-diff`, `merge -s octopus`
- Missing probes: `apply`, `ls-remote`, `prune` → generic not-implemented
- arks-only probes: `forge`/`host-env` respond; `lease`/`lock` require subcommand; `celir` not-implemented

## Deliverables

| Path | Content |
|------|---------|
| `.spec/research/20261111034805-git-arks-cli-help-crosscheck.research.md` | This note |
| `.spec/evidence/20261111034600-git-arks-cli-help-crosscheck/matrix-pin-git-vs-arks.csv` | Full command matrix |
| `.../arks-only-or-unlisted.csv` | arks-only / refuse arms / helpers |
| `.../spotcheck-high-traffic.txt` | Flag spot-checks |
| `.../pin-2.39.5-list-cmds.txt` | Pin inventory |
| `.../arks-help-cmds.txt` / `arks-dispatch-cmds.txt` | arks surfaces |

## Record

| Date (UTC) | Note |
|------------|------|
| 2026-11-10 | Exhaustive command-level HELP cross-check vs pin 2.39.5; Claimed=0; Missing=70 vs M16 narrative tension recorded honestly. |
