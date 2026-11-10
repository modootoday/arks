# Complete CLI flag symmetry matrix (G21) — M16 Phase Sy

Owning SoT: `.spec/sot/cli-surface.sot.md`, `.spec/sot/cli-flag-parity-complete.sot.md`, `.spec/sot/interop-test-matrix.sot.md` (G21). Plan: `.spec/plan/20261109043910-m16-complete-cli-flag-symmetry.plan.md`.

Authoritative venue: Docker clean-room service `g021-cli-flag-symmetry` (`verify/scripts/run-g021-cli-flag-symmetry.sh`). Host greens are advisory only.

## Wave Sy-1 claim set (everyday porcelain flags)

| ID | Condition | Required |
|----|-----------|----------|
| M16-SY1-01 | `status -b` prints branch header; with `--porcelain` emits `## branch` line | yes |
| M16-SY1-02 | `log --oneline` and `log --graph --oneline` emit peer-readable history; stock `git log` reads same repo | yes |
| M16-SY1-03 | `branch -a` / `branch -r` list local + remote-tracking as claimed | yes |
| M16-SY1-04 | `add --chmod=+x` / `--chmod=-x` updates index mode; stock Git sees mode | yes |
| M16-SY1-05 | `clean -x -f` removes ignored untracked; `describe --dirty` appends dirty mark | yes |
| M16-SY1-06 | `switch --detach` / `diff --stat` / `init --bare` claimed forms peer with stock Git | yes |
| M16-SY1-07 | G14 + G20 regression stay green | yes |

## Wave Sy-2 claim set

| ID | Condition | Required |
|----|-----------|----------|
| M16-SY2-01 | `worktree list --porcelain`; `worktree lock`/`unlock` | yes |
| M16-SY2-02 | `submodule update --recursive` / `status --recursive` on nested fixture | yes |
| M16-SY2-03 | `checkout --orphan` creates orphan branch | yes |

## Wave Sy-3 claim set

| ID | Condition | Required |
|----|-----------|----------|
| M16-SY3-01 | `update-index` / `read-tree` / `diff-tree` / `for-each-ref` / `rev-list` / `show-ref` peer subset | yes |
| M16-SY3-02 | `pack-objects` + `index-pack` round-trip vs stock Git | yes |

## Wave Sy-4 claim set

| ID | Condition | Required |
|----|-----------|----------|
| M16-SY4-01 | `clone`/`fetch`/`push` over local `ssh://` fixture (OpenSSH loopback) peer with stock Git | yes |
| M16-SY4-02 | `submodule add` over SSH URL fixture | yes |

## Wave Sy-5 claim set

| ID | Condition | Required |
|----|-----------|----------|
| M16-SY5-01 | `stash show`/`apply`/`drop`; cherry-pick `A..B` clean range | yes |
| M16-SY5-02 | `bisect run`; `notes append`; `blame` path; `fsck` / `count-objects` / `pack-refs` | yes |
| M16-SY5-03 | Detached `commit` / `commit --amend`; sequencer `--continue`/`--abort` on conflicted cherry-pick | yes |
| M16-SY5-04 | `format-patch --cover-letter`; `am --3way` when blobs resolvable | yes |
| M16-SY5-05 | `arks version` and `arks --version` print matching `arks version <semver>` | yes |

## Evidence schema

Receipt cites: image digest, stock git version, revision, per-ID pass, tip SHA.
