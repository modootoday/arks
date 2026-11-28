---
id: research:root/20261128034500
title: Full PR and branch audit and agy parallel-development governance analysis
status: active
created: 2026-11-28T03:45:00Z
updated: 2026-11-28T03:45:00Z
tags: [research, governance, audit, pull-requests, branches, agy, parallelism, concurrency]
verify: manual
rel:
  depends_on:
    - sot:root/autonomous-operation
    - sot:root/agent-governance
    - adr:root/20261119220000
    - adr:root/20261124000000
  related:
    - adr:root/20261128035000
    - sot:root/agy-automated-development
---

# Full PR and branch audit and `agy` parallel-development governance analysis

Survey date: **2026-11-28 UTC**. Tip surveyed on `origin/main`: **`aad381f`** (PR #74, `2026-11-25T06:45Z`).
Scope: process audit of all pull requests, branches, issues, rulesets, and runtime governance assumptions in `modootoday/arks`. This note makes no product Claimed assertion.

## 1. Repository pull-request inventory (complete)

Queried via `gh pr list --state all --limit 500` on 2026-11-28:

| Metric | Count / Value |
|--------|---------------|
| Total pull requests | **73** (#1–#74; #45 is a closed `needs-owner` issue, not a PR) |
| Open pull requests | **0** |
| Closed (unmerged) pull requests | **0** |
| Merged pull requests | **73** (100% merged via merge commit) |
| Distinct `headRefName` values across all 73 PRs | **1** (`claude/kind-faraday-uvczj4`: 73 / 73) |
| PRs carrying `needs-human` label | **9** (#2, #3, #4, #6, #7, #28, #43, #48, #53) |
| Revert PRs | **1** (#10 reverting #9; 91 files, +4570/-80 then -4570/+80) |
| Open issues / closed issues | **0** open / **1** closed (#45) |
| Git tags | **0** |

**Key finding:** Every merged PR in repository history flowed through a single reused branch (`claude/kind-faraday-uvczj4`), which GitHub auto-deleted when PR #74 merged (`delete_branch_on_merge: true`).

## 2. Remote and local branch inventory (complete)

Queried via `gh api repos/modootoday/arks/branches --paginate` and `git ls-remote origin`:

| Branch | Protected | Tip SHA | Last Commit (UTC) | Ahead / Behind `main` | Linked PR | Status |
|--------|-----------|---------|-------------------|-----------------------|-----------|--------|
| `main` | Yes (`main-protected`) | `aad381f` | 2026-11-25T06:45Z | 0 / 0 | — | Healthy; all 7 required checks green |
| `claude/dazzling-knuth-l8m1nz` | No | `9578d23` | 2026-11-25T02:06Z | **+9 / -152** | **None** | **Orphaned unmerged branch** |
| `claude/happy-volta-8xfk6m` | No | `890f116` | 2026-11-25T01:21Z | **+6 / -152** | **None** | **Orphaned unmerged branch** |

Local clone state: single branch `main` at `aad381f`, 1 worktree, 0 stashes.

### 2.1 Detailed inspection of orphaned remote branches

Both orphaned branches diverged from the exact same merge-base **`9abc6ed`** (`Merge pull request #47`, `2026-11-25T00:40Z`) and were pushed during the `2026-11-25T01:21Z–02:06Z` parallel session burst (concurrent with PRs #49–#56):

1. **`claude/dazzling-knuth-l8m1nz`** (`9578d23`, +9 commits, +653/-48 across 6 files):
   - Scope: `git format-patch` pin-2.39.5 parity fixes and corpus coverage:
     - Q-encoding of non-ASCII headers and 8-bit declaration (`crates/arks/src/cmd/format_patch/mail_header.rs`, +321)
     - Zero-padding patch numbers past nine patches (`format_patch/mod.rs`, renamed from `format_patch.rs`)
     - Wrapping cover-letter shortlog entries like `git shortlog` (`crates/arks/src/cmd/format_patch/wrap.rs`, +109)
     - Sending cover letter from committer even with `--from`; thread headers (`In-Reply-To` / `References`)
     - Corpus additions in `verify/fixtures/cli-flag-corpus/build-fixture.sh` (+27) and `family-commit.txt` (+48)
   - Overlap with `main` since `9abc6ed`: **0 commits** touched any of the 6 files on `main`. None of these commits landed on `main`.
   - Recommended disposition (owner decision): salvage by rebasing onto `main` in a dedicated `agy/fix/format-patch-headers-and-threading` PR (verified against fast gate and Docker corpus), then owner deletes the legacy remote branch.

2. **`claude/happy-volta-8xfk6m`** (`890f116`, +6 commits, +667/-103 across 10 files):
   - Scope: network `push` and `receive-pack` spec-first parity fixes:
     - Spec-first update in `.spec/sot/arks-remote.sot.md` (+4/-2)
     - List refs advertised by a `receive-pack` peer (`crates/arks-remote/src/ls_remote.rs`)
     - Return per-ref push results from `report-status` (`crates/arks-remote/src/push.rs`)
     - Resolve short ref names against network remote refs; format push status table and shorten source ref (`crates/arks/src/cmd/push.rs`, `remote_local.rs`, `pack_transport.rs`)
     - Integration test `crates/arks/tests/push_http_cli.rs` (+243)
   - Overlap with `main` since `9abc6ed`: **9 of 10 files have 0 commits** on `main`; only `crates/arks-remote/src/tests.rs` has 2 commits since `9abc6ed` (additive). None of these commits landed on `main`.
   - Recommended disposition (owner decision): salvage by rebasing onto `main` in a dedicated `agy/fix/push-report-status-and-refs` PR, then owner deletes the legacy remote branch.

## 3. Repository merge settings and ruleset baseline

- **Repository merge configuration:** `allow_merge_commit: true`, `allow_squash_merge: false`, `allow_rebase_merge: false`, `allow_auto_merge: true`, `delete_branch_on_merge: true`, `allow_update_branch: false`.
- **Ruleset `main-protected` (id `27672739`):** active on `~DEFAULT_BRANCH`; forbids deletion and force-push; requires pull request with merge method `merge` only (`required_approving_review_count: 0`, `require_extra_approval_for_unattributed_changes: true`); `strict_required_status_checks_policy: false`; 7 required checks: `lint (fmt + clippy)`, `test (ubuntu-24.04)`, `test (windows-2025)`, `test (macos-15)`, `msrv (1.85)`, `History`, `Public surface`.

## 4. Root-cause diagnosis: why parallel PR workflow broke down

1. **Single-branch serialization assumption vs actual multi-session concurrency:**
   - `autonomous-operation.sot.md` (v2.0.0) §4.1 and §7.1 mandated one topic branch (`claude/<name>`) reused sequentially via post-merge reset. When multiple sessions ran concurrently on 2026-11-25, two sessions pushed separate `claude/*` branches without opening PRs and were left invisible behind 152 commits while the primary branch kept rolling through PRs #48–#74.
2. **Hot-file collision and rebase amplification across concurrent PRs:**
   - Under H1–H7 (`adr:root/20261119220000`), topic branches must stay linear (no merge commits inside the branch; rebase onto `main` to resolve conflicts). Concurrent PRs modifying shared registry/dispatch/corpus/plan files (`crates/arks/src/cmd/mod.rs`, `git_dispatch.rs`, `Cargo.lock`, `build-fixture.sh`, `docs/completion.md`, `.spec/plan/*`, `production-residual-register.sot.md`) trigger repeated rebases, `--force-with-lease` pushes, and full 3-OS CI reruns.
3. **Runtime lock-in to Claude Code and `playpilot` local CLI:**
   - Prior governance depended on `.claude/settings.json` (`PreToolUse` `playpilot guard`), `playpilot mail inbox`, `playpilot sync`, and `playpilot claim` against a private fleet configuration repository.
   - In **`agy` (Antigravity)** automated sessions (remote Linux VMs or desktop sessions), `.claude/settings.json` hooks do not execute and neither `playpilot` nor `spec` is preinstalled by default, while GitHub (`gh` via `/opt/antigravity/bin/gh`), `git`, Rust, Docker, and native subagent orchestration (`invoke_subagent`) are directly available.
4. **Absence of a branch-hygiene sweep and lane ownership contract:**
   - Nothing checked `git ls-remote --heads origin` against open PRs before starting new work, so orphaned branches accumulated silently with no triage owner.
5. **Subagent workspace constraint in remote `agy` VMs:**
   - Remote `agy` subagents inherit the parent workspace (`inherit` mode); without explicit disjoint path ownership or explicit scratch worktrees, parallel subagents risk clobbering the same files or staging mixed commits.

## 5. Governance design requirements for `agy` canonization

To eliminate parallel PR drift while keeping every arks safety invariant (`main-protected`, H1–H7, `Public surface`, Docker clean-room receipts, English-only, Spec-first, A0/A1/A2):

1. **Task-scoped, runtime-aware branch taxonomy:** `agy/<lane>/<slug>` (1 branch = 1 work package = 1 PR, deleted on merge; never reuse a branch across unrelated units).
2. **Two-tier parallelism (Hub-and-Spoke Integrator first; bounded disjoint lanes second):**
   - **Tier A (default):** one `agy` Integrator session coordinates parallel subagents on disjoint paths and lands **one cohesive PR** per wave.
   - **Tier B (cross-session):** up to 3 concurrent agent PRs repository-wide ONLY on disjoint ownership lanes with an explicit **Hot-File Lock List** reserved for serialized/Integrator edits.
3. **Zero-Orphan Branch Invariant + mandatory Step 0 sweep:** every pushed `agy/*` branch MUST open a Draft or Ready PR in the same turn; every session begins by sweeping remote branches vs open PRs.
4. **GitHub-native control plane + portable local fast gate:** use GitHub Branches, PRs, Labels, Issues, CI checks, and `.spec/plan/` as the coordination bus when `playpilot` is absent; run a self-contained pre-push gate locally before every push so CI `History` and `Public surface` pass on the first run.
5. **Preserve strict A0 / A1 / A2 autonomy boundaries:** map A1 to `needs-human` + PR measurement and A2 to `needs-owner` GitHub issues (and Korean chat briefings in interactive sessions).
