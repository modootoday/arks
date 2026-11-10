---
id: plan:root/20261108183812
title: M9 CLI-parity production exit
status: active
created: 2026-11-08T18:38:12Z
updated: 2026-11-08T15:30:00Z
tags: [milestone, m9, phase-pc, cli, porcelain, parity, production, alias, dot-git]
rel:
  depends_on:
    - sot:root/program-roadmap
    - plan:root/20261108183811
    - sot:root/cli-surface
    - sot:root/public-surface
    - sot:root/remote-host-compatibility-surface
    - sot:root/interop-test-matrix
    - sot:root/product-verification-clean-room
    - sot:root/arks
    - adr:root/20261108083507
    - research:root/20261108183800
    - evidence:root/20261108143000
    - sot:root/clean-room-git-pin-advance
  implements: [sot:root/program-roadmap]
---

# M9 — Porcelain parity C + inventory closure (Phase Pc) — CLI-parity production

## Goal

Close remaining [[sot:root/cli-surface]] parity inventory, **flag-parity audit**, and **alias-ready audit** (`alias git=arks`); confirm **same `.git`** interchange; satisfy [[sot:root/program-roadmap]] **CLI-parity production** bar.

## Depends on

**M8** exit proven — [[evidence:root/20261108143000]] (G13 + G12/G7/G6 regression; gix-exit). Implementation tree SHA `885bb544961e1d74beb700c2921a18ad7a3ec53b` (evidence tip / HEAD at M8 close `11e1028458c15a008217b01afcd0077eb57bc944`).

## Frozen Wave Pc checklist (implement / refuse / exclude)

Primary cites: [git(1)](https://git-scm.com/docs/git); per-command man pages for clean-room pin (Git 2.39.x in `verify/images.lock.json`). Classification before G14 claim. Matrix: `verify/matrices/matrix-m9.md`. Flag-parity ledger: `.spec/sot/cli-flag-parity-m9.sot.md` (or appendix in unit SoT). Alias how-to: `docs/alias-git-arks.md`.

### Pc implement / deepen

| Command | Class | Claimed forms (M9) | Refuse / notes |
|---------|-------|--------------------|----------------|
| `tag` | **deepen** | Retain lightweight; add annotated `tag -a -m <msg> <name> [<commit>]` writing a `tag` object + `refs/tags/<name>` | Signed `-s` / `-u`; unclaimed long options |
| `archive` | **implement** | `archive [--format=tar] [-o <file>] [<tree-ish>]` — ustar of claimed tree (default HEAD) for regular files from tree walk | `zip`; `--remote`; `tar.gz` unless claimed; executable-bit best-effort |
| `reflog` | **implement** | `reflog` / `reflog show [<ref>]` — print `.git/logs/<ref>` lines when present; **append** simple reflog lines on claimed `commit` / branch tip `update-ref` / `checkout`/`switch` / `reset` / `merge` FF / claimed history edits that move HEAD | `expire` / `delete` / `exists`; missing log → empty success or honest empty |
| `bundle` | **deepen** | Retain create/list-heads/unbundle; add `bundle verify <file>` (header + reachable sanity for claimed format) | Thin-pack edges unclaimed |
| `clean` | **deepen** | Retain `-f` files; add `-d` remove untracked directories (non-ignored) | `-x` ignored still refuse |
| Wave L+H+Pa+Pb set | **retain** | Prior G6/G7/G12/G13 inventory remains green | Do not regress G13 / G12 / G7 |

### Pc refuse-closed (omit from `--help`; non-zero; no mutation)

| Command / form | Class | Rationale |
|----------------|-------|-----------|
| `worktree` | **refuse (M9 freeze)** | Multi-worktree deferred at M9 close; **post-M9 residual implement** [[plan:root/20261109022956]] — do not rewrite this M9 exit |
| `repack` | **refuse** | Pack maintenance via other paths; refuse-closed (not required for alias daily-driver) |
| `sparse-checkout` | **refuse** | Cone/non-cone sparse depth out of M9 claim |
| `submodule` | **refuse (M9 freeze)** | Nested repo deferred at M9 close; **post-M9 residual implement** [[plan:root/20261109022956]] — do not rewrite this M9 exit |
| `maintenance` | **refuse** | Background maintenance tasks out of M9 claim |
| `backfill` | **refuse** | Partial-clone backfill out of M9 claim (no partial-clone claim) |
| `range-diff` | **refuse** | History-comparison porcelain deferred |
| `notes` | **refuse** | Notes refs deferred |
| `filter-branch` | **refuse** | Deprecated; replacements not claimed |
| `rebase -i` / interactive | **refuse** | Retain Pb refuse |
| Non-FF `merge` / merge-style `pull` | **refuse** | Retain Pa/Pb; FF / `--rebase` only |
| `bisect run` / `skip` / `visualize` | **refuse** | Retain Pb refuse |
| Conflicted history edits | **refuse** | Retain Pb path-clean apply only |

### Product exclusions (not inventory rows to implement)

GUI (`gitk`/`gui`/`citool`), `scalar`, **Git LFS**, forge host APIs (B), Git **server** (**M11** / Wave Sv), CELIR, L1+ lease runtime (**M10**).

## Flag-parity audit (normative)

Every inventory command (L+H+Pa+Pb+Pc implement rows) MUST have a recorded row: **claimed flags** / **refuse flags** / **exclude**. Silent ignore of unknown `-` options is forbidden (`refuse_unknown_flags`). Audit artifact MUST be checked by G14 (file present + spot-check that help omits refuse-only commands and unknown flags fail).

## Alias-ready audit (normative)

1. Document `alias git=arks` (POSIX) / equivalent Windows / PATH argv shim under `docs/alias-git-arks.md`.
2. Clean-room G14 runs representative workflows by invoking a `git`→`arks` shim (argv-equivalent; same args after shim name) for: `init` → `add`/`commit` → `status`/`log` → `branch`/`checkout` → remote `clone`/`fetch` or local peer → refuse path non-zero under shim.
3. Refuse under alias stays non-zero and does not mutate the repo.

## Same-`.git` confirmation

Stock Git MUST open arks-written repos for claimed surfaces (and vice versa where tests assert). No parallel proprietary store. G14 includes at least one stock-Git peer open after arks annotated-tag + archive + reflog write.

## Production exit checklist (normative for M9)

- [x] Parity inventory closed against clean-room Git pin (− exclusions) — this frozen table.
- [x] Flag-parity audit recorded; no silent ignores.
- [x] **Alias-ready:** clean-room runs claimed workflows with `git` aliased to `arks` (or argv-equivalent shim); refuse paths stay non-zero under alias.
- [x] Docs explain how to set `alias git=arks` (or equivalent) and list exclusions / honesty limits.
- [x] Wave L + H + Pa + Pb + Pc help surfaces honest and complete for inventory.
- [x] G0–G7, G12–G14 green when harness exists (G8/G9 per policy/claims).
- [x] Workspace still **gix-exit** (zero gix edges) from M4 onward.
- [x] Public README/`docs/` may state CLI-parity / alias drop-in production **only** with receipts; no production-server / forge / LFS overclaim (server = **M11**).
- [x] **Same `.git`:** Stock Git still opens Git-compat mode repos for claimed surfaces; no parallel proprietary store.
- [x] CELIR not required; lease not required (runtime = **M10**).

Carrying receipt: [[evidence:root/20261108150000]].

## Exit gates

| Gate | Clean-room | Advisory |
|------|------------|----------|
| **G14** cli-parity-closure | Authoritative (`verify/gates/g14` + `run-g14.sh`) | Host |
| G13 + G12 + G7 (+ G6 / prior) regression | Clean-room (embedded in `run-g14.sh`) | Host |
| Help honesty / refuse / alias | Clean-room G14 | Host |
| Zero `gix` | `run-g14.sh` metadata scan | — |
| Unit Specfold | `spec check` | — |

## Ordered work items

1. **Design** — Freeze Pc checklist (this section); matrix `verify/matrices/matrix-m9.md`; unit SoT [[sot:root/arks]] Wave Pc; flag-parity ledger; alias docs.
2. **Scaffold** — `cmd_pc` (or deepen Pa/Pb); G14 crate + compose service + `run-g14.sh` (LF).
3. **Implement** — annotated tag → archive → reflog append/show → bundle verify → clean `-d`; refuse-close remainder; alias shim docs.
4. **Verify** — G14 + full regression; production checklist; public-surface honesty; evidence receipt; **this milestone IS CLI-parity production**.

## Out of scope

- Calling **M10** / **M11** “the” production bar.
- Lease runtime; CELIR; server M11 product claims; forge APIs (B).
- Inventing Git semantics; `gix`; regressing G13/G12/G7.
- GUI / LFS / scalar unless SoT amendment.

## Open questions

- **Pin-advance (closed-as-process):** Post-M9 Git pin bumps MUST follow [[sot:root/clean-room-git-pin-advance]] (gap audit → ledger → G14 → new evidence). M9 receipt [[evidence:root/20261108150000]] applies only to pin 2.39.5 in `verify/images.lock.json`.
- **Credential helper / askpass (refuse-closed for M9 production):** Not claimed. Remotes requiring interactive credential helper / `GIT_ASKPASS` are out of the M9 smart-HTTP client production bar unless amended. See [[sot:root/cli-surface]] Open decisions.

## Cross-links

[[sot:root/program-roadmap]], [[sot:root/cli-surface]], [[sot:root/public-surface]], [[sot:root/interop-test-matrix]], [[sot:root/arks]], [[adr:root/20261108083507]], `verify/matrices/matrix-m9.md`, [[evidence:root/20261108143000]]
