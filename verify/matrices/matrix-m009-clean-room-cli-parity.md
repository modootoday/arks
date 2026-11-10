# M9 clean-room usage-condition matrix (G14 + G13/G12/G7 regression)

Roadmap **v0.3.1+** M9 (Porcelain parity C + inventory closure / Phase **Pc**) — **CLI-parity production**. Authoritative venue: Docker clean-room under `verify/` per [[sot:root/product-verification-clean-room]]. Image pin: `verify/images.lock.json` (`arks-verify-m1:0.1.0`). Depends on M8 [[evidence:root/20261108143000]]. Unit SoT: [[sot:root/arks]] Wave Pc + [[sot:root/cli-surface]]. Plan: [[plan:root/20261108183812]].

**Authoritative runs:** `verify/docker-compose.yml` service `g014-cli-parity-production` (`verify/scripts/run-g014-cli-parity-production.sh`). No CELIR; no L1+ lease runtime (**M10**); no server (**M11**); no forge APIs (B).

Frozen Wave Pc claim set (implement / refuse / exclude) is normative in the M9 plan checklist. G14 covers **implemented** Pc rows + inventory/flag/alias audits; refused commands must exit non-zero without mutating the repo (including under alias shim).

| ID | Condition | Gate | Automated | Entry |
|----|-----------|------|-----------|-------|
| M9-C01 | Help lists Wave L+H+Pa+Pb+Pc claimed commands (incl. residual `worktree`/`submodule`); omits then-refuse rows (`backfill`, `range-diff`, `filter-branch`). **Post-M9 / M15 Fs-3 residual:** `notes` / `sparse-checkout` / `repack` / `gc` / `maintenance` are claimed implement — M9 historical exit receipt frozen | G14 | yes | `verify/gates/g014-cli-parity-production` |
| M9-C02 | Unimplemented / refused options exit non-zero (honesty); flag-parity ledger present | G14 | yes | `verify/gates/g014-cli-parity-production` |
| M9-C03 | Annotated `tag -a -m`; stock Git reads tag object | G14 | yes | `verify/gates/g014-cli-parity-production` |
| M9-C04 | `archive --format=tar` (HEAD) produces readable tar of tree files | G14 | yes | `verify/gates/g014-cli-parity-production` |
| M9-C05 | `reflog` / `reflog show` after commit moves; stock Git may read `.git/logs` | G14 | yes | `verify/gates/g014-cli-parity-production` |
| M9-C06 | `bundle verify` on arks-created bundle | G14 | yes | `verify/gates/g014-cli-parity-production` |
| M9-C07 | `clean -d` removes untracked directory | G14 | yes | `verify/gates/g014-cli-parity-production` |
| M9-C08 | Alias/shim workflows: init→commit→branch→status under `git`→`arks` argv shim | G14 | yes | `verify/gates/g014-cli-parity-production` |
| M9-C09 | Refuse under alias stays non-zero for still-open/exclude rows (e.g. `range-diff`); claimed `worktree list` / `submodule status` / Fs-3 `repack -ad` succeed under shim when repo allows | G14 | yes | `verify/gates/g014-cli-parity-production` |
| M9-C10 | Same-`.git`: stock Git `cat-file`/`log` on arks-written repo after Pc ops | G14 | yes | `verify/gates/g014-cli-parity-production` |
| M9-C11 | G13 Wave Pb regression | G13 | yes | via `run-g014-cli-parity-production.sh` |
| M9-C12 | G12 Wave Pa regression | G12 | yes | via `run-g014-cli-parity-production.sh` |
| M9-C13 | G7 Wave H regression | G7 | yes | via `run-g014-cli-parity-production.sh` |
| M9-C14 | G6 Wave L regression | G6 | yes | via `run-g014-cli-parity-production.sh` |
| M9-C15 | Prior G1–G5 / G3 / G10 as harnessed in `run-g014-cli-parity-production.sh` | — | yes | via `run-g014-cli-parity-production.sh` |
| M9-C16 | Zero `gix` / `gitoxide` Cargo edges | — | yes | `run-g014-cli-parity-production.sh` metadata scan |
| M9-C17 | Unit Specfold `spec check` | — | yes | host / CI |
| M9-C18 | Alias how-to docs present (`docs/alias-git-arks.md`) | — | yes | `verify/gates/g014-cli-parity-production` path check |
| M9-C19 | Lease L1+ / CELIR / server / forge APIs (B) | — | no | Out of M9 (**M10**/**M11**) |

Legend: **G14** = Wave Pc inventory + flag-parity + alias-ready + same-`.git`; regressions embed G13/G12/G7/G6 (+ prior).

Host-only green does **not** carry M9 exit. M9 exit **is** CLI-parity production when this matrix is green with a new evidence receipt.

## Residual honesty (post-M9 freeze)

| Residual | Disposition |
|----------|-------------|
| Pin-advance | Process: [[sot:root/clean-room-git-pin-advance]] — not a silent claim carry |
| Credential-helper / askpass | Refuse-closed for M9 production claims |
| Pc inventory / flag / alias / same-`.git` | Closed by G14 + ledger; re-smoke via residual hygiene regression |
| `worktree` / `submodule` | Post-M9 residual implement [[plan:root/20261109022956]]; M9 historical refuse freeze intact |
| Register | [[sot:root/production-residual-register]] |

## Honesty residuals (client)

| ID / topic | Status | Notes |
|------------|--------|-------|
| Pin-advance | Process closed | [[sot:root/clean-room-git-pin-advance]]; receipt [[evidence:root/20261108150000]] bound to `verify/images.lock.json` Git 2.39.5 |
| Credential helper / askpass | Refuse-closed (M9) | Not in matrix rows; not claimed for production smart-HTTP client |
| M9-C19 | Out of scope | Lease (**M10**), CELIR, server (**M11**), forge (B) |
