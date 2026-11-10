# Full CLI symmetric parity matrix (G20) — M15 Phase Fs

Owning SoT: `.spec/sot/cli-surface.sot.md`, `.spec/sot/cli-flag-parity-full.sot.md`, `.spec/sot/interop-test-matrix.sot.md` (G20). Plan: `.spec/plan/20261109035335-full-cli-symmetric-parity.plan.md`.

Authoritative venue: Docker clean-room service `g20` (`verify/scripts/run-g20.sh`). Host greens are advisory only.

## Wave Fs-1 claim set

| ID | Condition | Required |
|----|-----------|----------|
| M15-FS1-01 | `commit --amend -m` replaces HEAD; stock Git reads amended commit | yes |
| M15-FS1-02 | Sync `pre-commit` hook runs and can abort commit; `--no-verify` skips | yes |
| M15-FS1-03 | `merge --no-ff` on FF-capable histories creates 2-parent merge commit | yes |
| M15-FS1-04 | Plumbing: `write-tree` / `commit-tree` / `ls-files` / `ls-tree` / `symbolic-ref` / `merge-base` peer with stock Git on fixture | yes |
| M15-FS1-05 | G14 regression (historical M9 inventory) stays green | yes |

## Wave Fs-2 claim set

| ID | Condition | Required |
|----|-----------|----------|
| M15-FS2-01 | Diverged non-overlapping histories: `merge` creates 2-parent commit; both sides' files present; stock `git fsck` OK | yes |
| M15-FS2-02 | Diverged overlapping edit: merge fails non-zero; conflict markers in worktree | yes |

## Wave Fs-3 claim set (former refuse-closed porcelain)

| ID | Condition | Required |
|----|-----------|----------|
| M15-FS3-01 | `notes add -m` + `notes show` round-trip; stock `git notes show` reads arks note | yes |
| M15-FS3-02 | `sparse-checkout init --cone` + `set <dir>` + `list`; cone patterns on disk; skip-worktree applied outside cone | yes |
| M15-FS3-03 | `repack -ad` under `maint:gc` lease packs reachable objects; stock `git fsck` OK; concurrent foreign `maint:gc` holder → non-zero | yes |
| M15-FS3-04 | `gc` consolidates under lease; `gc --prune=now` refuses (CORR-001 honesty) | yes |
| M15-FS3-05 | `filter-branch` / `backfill` explicit exclude (non-zero honesty); not silent ignore | yes |

## Wave Fs-4 claim set (flag gaps; may land before Fs-3 refuse-closed)

| ID | Condition | Required |
|----|-----------|----------|
| M15-FS4-01 | `add -A` stages new+modified; `status --porcelain` emits XY lines; stock `git status --porcelain` succeeds on same repo | yes |

## Wave Fs-5 claim set (interactive honesty + range-diff)

| ID | Condition | Required |
|----|-----------|----------|
| M15-FS5-01 | `range-diff A..B C..D` pairwise subject compare (`=`/`!`); stock Git may peer-read same ranges | yes |
| M15-FS5-02 | `commit -a -m` stages tracked updates and commits | yes |
| M15-FS5-03 | Interactive/editor forms explicit exclude: `rebase -i` and `commit` without `-m` refuse with exclude honesty | yes |

## Later waves

None for M15 Phase Fs after Fs-5 (exit = all claimed waves green under G20).
