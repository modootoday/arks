# M8 clean-room usage-condition matrix (G13 + G12/G7 regression)

Roadmap **v0.3.1+** M8 (Porcelain parity wave B / Phase **Pb**). Authoritative venue: Docker clean-room under `verify/` per [[sot:root/product-verification-clean-room]]. Image pin: `verify/images.lock.json` (`arks-verify-m1:0.1.0`). Depends on M7 [[evidence:root/20261108133000]]. Unit SoT: [[sot:root/arks]] Wave Pb + [[sot:root/cli-surface]]. Plan: [[plan:root/20261108183811]].

**Authoritative runs:** `verify/docker-compose.yml` service `g13` (`verify/scripts/run-g13.sh`). **Not production** (production = **M9**). No CELIR; no L1+ lease runtime (**M10**); no server (**M11**); no forge APIs (B).

Frozen Wave Pb claim set (implement / refuse / exclude) is normative in the M8 plan checklist. G13 covers **implemented** rows only; refused commands must exit non-zero without mutating the repo.

| ID | Condition | Gate | Automated | Entry |
|----|-----------|------|-----------|-------|
| M8-C01 | Help lists Wave L+H+Pa+Pb claimed commands; omits deferred (`range-diff`, `notes`, `filter-branch`, plus Pa deferred) | G13 | yes | `verify/gates/g13` |
| M8-C02 | Unimplemented / refused options exit non-zero (honesty) | G13 | yes | `verify/gates/g13` |
| M8-C03 | `cherry-pick` single clean commit onto HEAD; stock Git opens repo | G13 | yes | `verify/gates/g13` |
| M8-C04 | `revert` single clean commit; stock Git opens repo | G13 | yes | `verify/gates/g13` |
| M8-C05 | Non-interactive `rebase <upstream>` linear replay | G13 | yes | `verify/gates/g13` |
| M8-C06 | `format-patch` + `am` round-trip for claimed single/series patches | G13 | yes | `verify/gates/g13` |
| M8-C07 | `bisect` start/bad/good/reset on first-parent | G13 | yes | `verify/gates/g13` |
| M8-C08 | `bundle` create `--all` / list-heads / unbundle | G13 | yes | `verify/gates/g13` |
| M8-C09 | `pull --rebase` when remote tracking exists | G13 | yes | `verify/gates/g13` |
| M8-C10 | Conflicted cherry-pick/rebase refuses non-zero (no silent rewrite) | G13 | yes | `verify/gates/g13` |
| M8-C11 | G12 Wave Pa regression | G12 | yes | via `run-g13.sh` |
| M8-C12 | G7 Wave H regression | G7 | yes | via `run-g13.sh` |
| M8-C13 | G6 Wave L regression | G6 | yes | via `run-g13.sh` |
| M8-C14 | Prior G1–G5 / G3 / G10 as harnessed in `run-g13.sh` | — | yes | via `run-g13.sh` |
| M8-C15 | Zero `gix` / `gitoxide` Cargo edges | — | yes | `run-g13.sh` metadata scan |
| M8-C16 | Unit Specfold `spec check` | — | yes | host / CI |
| M8-C17 | `range-diff` / `notes` / interactive rebase / annotated tags | — | no | Wave **Pc** (M9) or refuse |
| M8-C18 | Inventory / flag-parity / alias-ready closure | — | no | Wave **Pc** (M9) |
| M8-C19 | Lease L1+ / CELIR / server / forge APIs (B) | — | no | Out of M8 |

Legend: **G13** = Wave Pb porcelain argv + on-disk peer oracle; regressions embed G12/G7/G6 (+ prior).

Host-only green does **not** carry M8 exit.
