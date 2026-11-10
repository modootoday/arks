---
title: Contribute
description: How to contribute to arks — bugs, docs, and maintainer paths
sections:
  - humans
  - docs
  - maintainers
  - {c: cta, name: next}
---

# Contribute

Bug reports and pull requests are welcome on [modootoday/arks](https://github.com/modootoday/arks).

## For humans

- Prefer English in product docs, issues, and PR descriptions.
- Keep secrets out of the tree (tokens, `.env`, private keys, unredacted private traces).
- Say what you tried with the **native** `arks` CLI (`arks --version` / build tip) when reporting bugs.
- If the bug is multi-session contention, say how many writers / worktrees / remotes you used — without pasting credentials.

## Docs on this site

Human-facing Pages live under `docs/`. Prefer short product language over dumping design specs. Lead with the multi-session contention motive when it helps; do not turn Pages into an agent-governance ledger. Design specs and verification matrices stay in `.spec/` and `verify/` for maintainers.

## For maintainers and agents

Start from [`AGENTS.md`](https://github.com/modootoday/arks/blob/main/AGENTS.md). Clean-room Docker runs prove claims; they are not the end-user install path. See [Performance](../performance/) and [Concurrent sessions](../concurrent-sessions/) for how verification framing is explained publicly.

## Next

[Getting started](../getting-started/) [Limits](../limits/) [Source on GitHub](https://github.com/modootoday/arks)
