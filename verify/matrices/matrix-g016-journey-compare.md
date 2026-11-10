# Journey-compare matrix (G16)

Owning SoT: `.spec/sot/clean-room-journey-compare.sot.md` **v0.4.0**. Plan: `.spec/plan/20261108182457-clean-room-journey-compare.plan.md`. Residual worktree/submodule: `.spec/plan/20261109022956-worktree-submodule-residual.plan.md`.

Authoritative venue: Docker clean-room service `g016-journey-compare` (`verify/scripts/run-g016-journey-compare.sh`). Host runs are advisory.

| ID | Sketch | Gate | Required | Harness |
|----|--------|------|----------|---------|
| J-LOCAL-01 | `init` → `add`/`commit` → `status`/`log` | G16 | yes | `verify/gates/g016-journey-compare` |
| J-BRANCH-01 | `branch` / `checkout` / `switch` | G16 | yes | `verify/gates/g016-journey-compare` |
| J-MERGE-FF-01 | `merge --ff-only` | G16 | yes | `verify/gates/g016-journey-compare` |
| J-REBASE-01 | non-interactive `rebase` | G16 | yes | `verify/gates/g016-journey-compare` |
| J-BUNDLE-01 | `bundle create` / `verify` | G16 | yes | `verify/gates/g016-journey-compare` |
| J-CLONE-LOCAL-01 | smart-HTTP local `clone` | G16 | yes | `verify/gates/g016-journey-compare` |
| J-CLONE-PATH-01 | path + `file://` local clone | G16 | yes | `verify/gates/g016-journey-compare` |
| J-ALIAS-01 | J-LOCAL under `git`→`arks` shim | G16 | yes | `verify/gates/g016-journey-compare` |
| J-WORKTREE-01 | `worktree add`/`list`/`move`/`remove`/`prune`; lock fail-closed | G16 | yes | `verify/gates/g016-journey-compare` |
| J-TAG-01 | annotated `tag -a -m` | G16 | yes | `verify/gates/g016-journey-compare` |
| J-STASH-01 | `stash push` → `stash list` → `stash pop` on dirty worktree | G16 | yes | `verify/gates/g016-journey-compare` |
| J-CHERRY-01 | single clean `cherry-pick` | G16 | yes | `verify/gates/g016-journey-compare` |
| J-DIFF-01 | `diff` / `diff --cached` | G16 | yes | `verify/gates/g016-journey-compare` |
| J-RM-MV-01 | `mv` / `rm` tracked | G16 | yes | `verify/gates/g016-journey-compare` |
| J-ARCHIVE-01 | `archive --format=tar` | G16 | yes | `verify/gates/g016-journey-compare` |
| J-REFLOG-01 | `reflog` / `reflog show` | G16 | yes | `verify/gates/g016-journey-compare` |
| J-REVERT-01 | single clean `revert` | G16 | yes | `verify/gates/g016-journey-compare` |
| J-REPACK-01 | seed → `repack -ad` (M15 Fs-3 lease-honest pack) | G16 | yes | `verify/gates/g016-journey-compare` |
| J-SUBMODULE-01 | `submodule add`/`status`/`deinit`/`init`/`update` (local); gitlink + `.gitmodules` | G16 | yes | `verify/gates/g016-journey-compare` |

**Retired:** J-REFUSE-01 (`worktree list` refuse) and J-REFUSE-03 (`submodule status` refuse) — superseded by implement rows ([[plan:root/20261109022956]]). **J-REFUSE-02** (`repack -ad` refuse) — superseded by **J-REPACK-01** ([[plan:root/20261109035335]] Fs-3). Historical G16 receipts that recorded refuse PASS remain valid for their tree SHAs; current catalog requires implement PASS.
