---
id: plan:root/20261120000000
title: argv symmetry ladder (A0–A_final) — every command and argv symmetric with git, gh and git daemon
status: active
created: 2026-11-20T00:00:00Z
updated: 2026-11-20T03:00:00Z
tags: [cli, argv, symmetry, flags, git, gh, daemon, inventory, plan, no-invent-pass]
verify: manual
rel:
  depends_on:
    - sot:root/cli-surface
    - sot:root/cli-git-help-matrix
    - sot:root/arks-hub-gh-command-matrix
    - sot:root/arks-cli-personality-factory
    - adr:root/20261119230000
    - plan:root/20261119210000
---

# argv symmetry ladder — execution plan

## Directive (owner, 2026-11-20)

Every `arks` command **and its argv** MUST be symmetric with its peer: stock `git` (pin
**2.39.5**), `gh` (pin **2.103.3**, rows in scope per [[adr:root/20261119230000]]) and
`git daemon`. "Symmetric" means the same command names, the same option spellings (short,
long, `--no-` negations, `=`/separate values, bundles) and the same observable result
(stdout, exit code, resulting repository state).

**Refused git commands are in scope too (owner decision, 2026-11-20: "implement all").** The
47 rows of `verify/matrices/cli-git-help-matrix.csv` that were `refuse-with-reason` (git-internal
helpers, merge-strategy programs, GUI, Windows-only, mail, external-tool bridges) become
`must-implement` ([[sot:root/cli-git-help-matrix]] rule 3 amended).

## Measurement

1. **Acceptance inventory** — `verify/scripts/run-argv-inventory.py <arks> <out>` reads each
   command's options from stock `git <cmd> -h` and runs `arks <cmd> <option>` in a fresh
   fixture copy; an option is REFUSED when arks reports it unsupported/unknown. Parse level
   only.
2. **Behaviour corpus** — `verify/scripts/run-cli-flag-corpus-compare.sh` (tiers A/B, plus new
   per-family tiers) compares stdout, exit code and repository state with stock git.
3. Claimed only from the Docker clean-room image (pin git 2.39.5); host runs are advisory.

## Baseline (A0, Docker pin 2.39.5, tip `5e55cac`)

[[evidence:root/20261120000500-argv-inventory-baseline]]: 138 `must-implement` commands;
101 have a `-h` option list with **1,776** options, of which arks accepts **557** (31%); 12
commands accept every listed option; 37 commands print no option list (rev-list / diff family,
sub-command groups, helpers) and need a documentation- or completion-derived inventory.

## Rungs

| Rung | Scope | Exit |
|------|-------|------|
| **A0** | Inventory tool + baseline | Done (this plan) |
| **A1** | Inventory completion: per-subcommand `-h` (stash, notes, worktree, remote, submodule, bisect, sparse-checkout, maintenance, …) and documentation-derived lists for the rev-list / diff option families | Every must-implement command has an option list |
| **A2** | Porcelain waves by family — history/inspection (log, show, rev-list, diff*, grep, blame, …); commit/branch (commit, branch, tag, checkout, switch, restore, reset, add, rm, mv, clean, status, stash, merge, rebase, cherry-pick, revert, am, apply, format-patch); remote (clone, fetch, pull, push, ls-remote, remote, submodule); plumbing/maintenance (update-index, read-tree, pack-objects, repack, gc, config, refs, …) | Acceptance 100% per family + behaviour corpus lines SAME (reasoned DIFF listed) |
| **A3** | The 47 formerly refused commands, including merge-strategy programs, `difftool` / `mergetool`, `filter-branch`, `quiltimport`, `send-email` / `imap-send`, `instaweb`, credential helpers, GUI (`gui`, `citool`) and Windows-only rows | Each command dispatches with git's argv (see A3 design decisions) |
| **A4** | gh: flag-level inventory for in-scope rows from the gh 2.103.3 manual; implement `later` / `Deferred` rows (ruleset, `repo gitignore|license`, `pr checks`, `browse`, `repo clone|sync|read-*`, `search code|commits`, `issue develop|artifact`, `auth switch`) | gh-matrix in-scope rows v1 with gh-symmetric flags |
| **A5** | git daemon: review the 16 `Refuse/N/A` daemon rows | Daemon matrix without unreasoned refuse |
| **A_final** | Docker receipts for inventory + corpora; matrices and `docs/completion.md` match | — |

## A3 design decisions (owner, 2026-11-20)

1. **GUI rows** (`gui`, `citool`, `gui--askpass`, `gui--askyesno`, `gui.tcl`): git ships these as a
   Tcl/Tk program that arks cannot reuse. arks implements them as a **terminal UI** with git's
   command names and argv (`arks gui [blame|browser|citool|version]`, `arks citool`, askpass /
   askyesno prompts on the terminal). No windowing toolkit dependency.
2. **Windows-only rows** (`credential-manager`, `credential-wincred`, `update-git-for-windows`):
   implemented on every OS. The credential rows speak the credential-helper protocol
   (`get` / `store` / `erase`) backed by the OS credential store (Windows Credential Manager on
   Windows, the platform keychain or secret service elsewhere, with an honest refuse when none
   exists). `update-git-for-windows` checks for a newer `arks` release and never installs Git
   for Windows.
3. Every other former refuse row follows git's documented behaviour.

## Progress

- **A4 (host-advisory, not a receipt):** `verify/scripts/run-gh-argv-inventory.py` with the
  installed gh 2.89.0 as oracle (pin 2.103.3; newer leaves from
  `verify/matrices/gh-argv-supplement.csv`): baseline 264 / 1,268 flag spellings accepted
  (780 unvalidated: leaves that ignored unknown flags) →
  [[evidence:root/20261120015729-gh-argv-inventory-baseline]]; after A4 all 1,268 accepted and every
  leaf refuses unknown flags ([[evidence:root/20261120040000-gh-argv-inventory-a4]]). The formerly `later` rows (rulesets, template catalogs, `pr checks`
  over arks commit statuses, `browse`, `repo clone|sync|read-*`, `search code|commits`,
  `issue develop|artifact`, `auth switch`) are implemented against the forge and tested in crate
  tests plus `ARKS_HUB_IW_GH_ARGV=1` of the hub e2e script. Rows stay Partial until a Docker receipt.
- **A3 merge strategies and internal helpers (design):** the strategy programs (`merge-octopus`,
  `merge-resolve`, `merge-ours`, `merge-recursive[-ours|-theirs]`, `merge-subtree`) follow the
  `<base>... -- <head> <remote>...` convention with exit 0 / 1 / 2 (<https://git-scm.com/docs/git-merge#_merge_strategies>);
  `git merge` with several heads, `-s octopus|subtree` and `-X subtree[=<path>]` route through the
  same engines. `merge-index` runs its program per unmerged path; `git-merge-one-file` runs
  in-process, any other program is the user's. Helpers that pin 2.39.5 ships as internals mirror
  their observed behaviour: `pickaxe` is `blame`; `add--interactive`, `bisect--helper` and
  `submodule--helper` route to the `add -i|-p` / `bisect` / `submodule` bodies; `env--helper`,
  `sh-i18n--envsubst` and `upload-archive--writer` follow their documented contracts;
  `checkout--worker` speaks an empty request stream (arks checks out in-process);
  `fsmonitor--daemon` reports "not supported on this platform" where stock git (Linux builds) does.
  Rows absent from 2.39.5 mirror what git exposes: `update` is not a git command (same message as
  stock git); `backfill` fetches missing blobs of a partial clone (git 2.49 contract);
  `credential-helper-selector` asks on the terminal and forwards to the chosen helper.
  `credential-cache` / `credential-cache--daemon` are a native Unix-socket cache (refused honestly
  where Unix sockets are unavailable). `remote-ext`, `remote-fd` and `remote-http|https|ftp|ftps`
  speak gitremote-helpers(7) (<https://git-scm.com/docs/gitremote-helpers>) on stdin; the HTTP helpers delegate to arks's smart HTTP client, and
  `ftp|ftps` refuse at transfer time (no FTP client). Corpus: `family-a3-helpers.txt`.

## Order

A1 → A2 (families in parallel, integrated one family at a time) → A3 → A4 → A5 → A_final.
Work never invents PASS: rows stay Partial until a receipt shows them.
