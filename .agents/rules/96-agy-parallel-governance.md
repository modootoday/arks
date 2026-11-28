# agy parallel-development governance

Canon: `.spec/sot/agy-automated-development.sot.md`; decision `.spec/adr/20261128035000-agy-parallel-governance-and-branch-taxonomy.adr.md`; runbook skill `agy-parallel-orchestration`.

## Must

1. **Step 0 sweep first:** before starting new work, run `git fetch origin main`, `git ls-remote --heads origin`, and `gh pr list --state open` to detect red `main`, orphan remote branches, stale/conflicting PRs, and open PR count.
2. **Task-scoped branch taxonomy:** create one fresh branch `agy/<lane>/<slug>` (`lane` in `feat|fix|refactor|docs|test|verify|gov|int|revert`) from `origin/main` per work package or wave. Never reuse an old branch across unrelated PRs.
3. **Zero-orphan invariant:** every push of an `agy/*` branch MUST open a Draft or Ready PR in the same turn.
4. **Two-Tier parallelism:**
   - **Tier A (default):** one Integrator session fans out `invoke_subagent` workers on strictly disjoint paths, integrates serially, runs the portable fast gate once, and opens **one PR**. Subagents never push, open PRs, or merge.
   - **Tier B (cross-session):** at most **3 open agent PRs** repo-wide, strictly on disjoint Ownership Lanes (`L-core`, `L-cli`, `L-remote`, `L-hub`, `L-serve`, `L-agent`, `L-verify`, `L-gov`), with zero concurrent edits to shared hot files (`Cargo.toml`, `Cargo.lock`, `cmd/mod.rs`, `git_dispatch.rs`, `porcelain_opts.rs`, `build-fixture.sh`, roadmap/register/queue/completion/AGENTS).
5. **Portable pre-push gate:** before pushing, verify Rust `fmt`/`clippy`/`test` (when `crates/` or `Cargo.*` changed), H1–H7 commit history (Conventional Commits ≤72 chars, no merge commits, owner noreply identity, zero trailers), zero Hangul in tracked files/commits, zero secrets/private terms, and Specfold link/frontmatter integrity.
6. **PR Lease Header & Autonomy Levels:** include `Parallel Lease` (runtime, tier, lane, hot files, base tip), `Verification`, and `Class` (`A0` auto-merge, `A1` `needs-human` + measurement, `A2` owner-only via `needs-owner` issue / Korean chat).

## Must not

- Push `main`, tags, `claude/*`, or another session's `agy/*` branch.
- Leave a pushed `agy/*` branch without a linked Pull Request.
- Delete foreign or legacy remote branches without explicit owner authorization (A2).
- Modify a Hot File concurrently with another open PR, or open a 4th concurrent agent PR.
- Edit `.claude/settings*.json` or `.cursor/hooks*`, or weaken required CI checks / tests.
- Run concurrent Docker clean-room sessions on the same host (sole-runner lock applies).
