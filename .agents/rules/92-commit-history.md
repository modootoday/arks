# Commit history (playpilot H1–H7)

Decision: `.spec/adr/20261119220000-commit-history-playpilot-h1-h7.adr.md`. Canon: playpilot
`.agents/rules/commit-history.md` (rev `0ddc0e1`). Stricter arks rules (`27`, `28`, `05`, `91`) still apply.

## Must

1. Merge method is **merge commit**; every commit on a topic branch has a Conventional Commits subject
   (`type(scope): summary`, imperative, ≤72 chars). Spec-first → `docs(spec): …`.
2. One work package per PR; no merge / WIP / `fixup!` / `squash!` / `amend!` / empty commits at merge time;
   catch up with `main` by rebase.
3. Author and committer are the owner GitHub noreply identity; no session-link trailer; any
   `Co-authored-by` uses an `@users.noreply.github.com` address. Put tool/session attribution in the PR body.
4. Enable auto-merge right after opening a PR; disable it before pushing more commits, re-enable after.
5. `.github/` changes carry the `needs-human` label.

## Must not

- Rewrite `main`, tags, merged or shared history; force-push except `--force-with-lease=<branch>:<sha>`
  to your own unmerged `claude/*` branch.
- Merge a PR directly (auto-merge or the owner merges).
