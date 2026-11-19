---
id: adr:root/20261119220000
title: Commit history canon — adopt playpilot H1–H7 (merge commit)
status: active
created: 2026-11-19T22:00:00Z
updated: 2026-11-19T23:00:00Z
tags: [governance, commit-history, playpilot, merge-commit, conventional-commits, ci, auto-merge]
rel:
  depends_on:
    - sot:root/agent-governance
---

# Commit history canon — adopt playpilot H1–H7 (merge commit)

## Context

The owner adopted the playpilot commit-history canon across repositories (stage 1). Canon:
<https://github.com/modootoday/playpilot/blob/0ddc0e1dfd1bac378901817a5fb106935fd5fba1/.agents/rules/commit-history.md>
(H1–H7). arks rules stay in force where they are stricter (spec-first order, English only,
evidence honesty, push only to the session `claude/*` branch).

## Decision

1. **H1 merge method:** arks uses **merge commit** (PR #1 merged that way; Specfold receipts cite
   topic-branch commit hashes). The base-branch ruleset lists only that method.
2. **Subjects (H3/H6):** with merge commit every commit on a topic branch carries a Conventional
   Commits subject `type(scope): summary`, imperative, at most 72 characters, types
   `feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert`. Spec-first commits use
   `docs(spec): …`; evidence `docs(evidence): …`; verify harness `test(verify): …`.
3. **Topic branch hygiene (H3):** one work package per PR; no merge commits inside the branch
   (catch up by rebase); no WIP / `fixup!` / `squash!` / `amend!` / empty commits at merge time;
   lockfile or generated churn in its own `build` / `chore(deps)` commit.
4. **Identity and trailers (H3, C10.2/C10.3/C10.5):** author and committer are the owner GitHub
   noreply identity; no session-link trailer and no `Co-authored-by` with a non-noreply address.
   Tool / session attribution goes in the PR body. This replaces the per-commit attribution
   trailers previously used by agent sessions in this repository.
5. **Rewrites (H4):** base branch and tags are never rewritten. An agent may rewrite only its own
   unmerged `claude/*` branch to meet H3, pushing with `--force-with-lease=<branch>:<expected-sha>`;
   plain `--force` / `+refspec` stay forbidden (narrows rule `91`).
6. **Undo (H5):** revert merged changes with `git revert` in a new PR titled `revert: <subject>`.
7. **Auto-merge (H7):** the agent enables auto-merge right after opening a PR; to add commits it
   disables auto-merge, pushes, then re-enables. Agents never merge directly. `needs-human` PRs
   (e.g. `.github/` changes) follow the same flow; removing or weakening required checks,
   loosening rulesets, releases and tags stay with the owner.
8. **Enforcement stage 1:** CI job `History` runs
   `playpilot history-check --method merge --range "$BASE..$HEAD"` (playpilot pinned by rev) as a
   **non-required** check.
9. **Enforcement stage 2:** `.claude/settings.json` registers `playpilot guard` (pinned rev, installed
   on demand) as a `PreToolUse` hook for `Bash|PowerShell` with `PLAYPILOT_PROTECTED_BRANCHES=main`.
   Measured: push to `main` (incl. `HEAD:main`, `git -C`, `cd … &&`), `--force`, `--tags` refused
   (exit 2); topic-branch push and `--force-with-lease=<branch>:<full-sha>` allowed (exit 0).
   **Known gap:** the guard inspects only the `git` program, so `arks push origin main` (with or
   without `ARKS_PERSONALITY=git`) is not seen. Until playpilot covers `arks`, rule `92` forbids
   pushing protected branches with arks, and the base-branch ruleset stays the server-side backstop.

## Consequences

Commits before this ADR (including PR #1) keep their historical subjects and trailers; `History`
judges only new PR ranges.
