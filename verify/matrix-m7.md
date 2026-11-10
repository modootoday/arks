# M7 clean-room usage-condition matrix (G12 + G6/G7 regression)

Roadmap **v0.3.1+** M7 (Porcelain parity wave A / Phase **Pa**). Authoritative venue: Docker clean-room under `verify/` per [[sot:root/product-verification-clean-room]]. Image pin: `verify/images.lock.json` (`arks-verify-m1:0.1.0`). Depends on M6 [[evidence:root/20261108112000]]. Unit SoT: [[sot:root/arks]] Wave Pa + [[sot:root/cli-surface]]. Plan: [[plan:root/20261108183810]].

**Authoritative runs:** `verify/docker-compose.yml` service `g12` (`verify/scripts/run-g12.sh`). **Not production** (production = **M9**). No CELIR; no L1+ lease runtime (**M10**); no server (**M11**); no forge APIs (B).

Frozen Wave Pa claim set (implement / refuse / exclude) is normative in the M7 plan checklist. G12 covers **implemented** rows only; refused commands must exit non-zero without mutating the repo.

| ID | Condition | Gate | Automated | Entry |
|----|-----------|------|-----------|-------|
| M7-C01 | Help lists Wave L+H+Pa claimed commands; omits deferred (`worktree`, `repack`, `reflog` if refuse) | G12 | yes | `verify/g12` |
| M7-C02 | Unimplemented / refused options exit non-zero (honesty) | G12 | yes | `verify/g12` |
| M7-C03 | `branch` list/create/delete/rename claimed forms vs stock Git peer | G12 | yes | `verify/g12` |
| M7-C04 | `checkout` / `switch` branch switch + create; path restore subset | G12 | yes | `verify/g12` |
| M7-C05 | `restore` / `reset` (--soft/--mixed/--hard) claimed forms | G12 | yes | `verify/g12` |
| M7-C06 | `diff` WT/index/HEAD and commit-pair text unified subset | G12 | yes | `verify/g12` |
| M7-C07 | `merge` fast-forward; `--ff-only`; refuse unclaimed strategies/conflicts as documented | G12 | yes | `verify/g12` |
| M7-C08 | `rm` / `mv` / `clean -f` claimed forms | G12 | yes | `verify/g12` |
| M7-C09 | `tag` lightweight create/list/delete; `describe` / `shortlog` claimed subset | G12 | yes | `verify/g12` |
| M7-C10 | `config` get/set/list/unset local; `grep` worktree/tracked subset | G12 | yes | `verify/g12` |
| M7-C11 | `stash` push/list/pop minimal; `worktree` / `reflog` / `repack` refuse if deferred | G12 | yes | `verify/g12` |
| M7-C12 | `pull` = fetch + merge `--ff-only` when remote exists; refuse non-ff | G12 | yes | `verify/g12` |
| M7-C13 | G7 Wave H regression (smart HTTP + gix-exit) | G7 | yes | via `run-g12.sh` |
| M7-C14 | G6 Wave L regression | G6 | yes | via `run-g12.sh` |
| M7-C15 | Prior G1–G5 / G3 / G10 as harnessed in `run-g12.sh` | — | yes | via `run-g12.sh` |
| M7-C16 | Zero `gix` / `gitoxide` Cargo edges | — | yes | `run-g12.sh` metadata scan |
| M7-C17 | Unit Specfold `spec check` | — | yes | host / CI |
| M7-C18 | Annotated tags / GPG / rebase / cherry-pick / bisect | — | no | Wave **Pb** (M8) |
| M7-C19 | Inventory / flag-parity / alias-ready closure | — | no | Wave **Pc** (M9) |
| M7-C20 | Lease L1+ / CELIR / server / forge APIs (B) | — | no | Out of M7 |

Legend: **G12** = Wave Pa porcelain argv + on-disk peer oracle; regressions embed G6/G7 (+ prior).

Host-only green does **not** carry M7 exit.
