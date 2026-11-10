---
title: Contribute
description: How to contribute to arks — bugs, docs, and maintainer paths
nav_order: 80
sections:
  - {c: steps, name: humans}
  - docs
  - maintainers
  - {c: cta, name: next}
---

# Contribute

Bug reports and pull requests are welcome on [modootoday/arks](https://github.com/modootoday/arks).

## For humans

1. Prefer English in product docs, issues, and PR descriptions.
2. Keep secrets out of the tree (tokens, `.env`, private keys, unredacted private traces).
3. Say what you tried with the **native** `arks` CLI (`arks --version` / build tip) when reporting bugs.
4. If the bug is multi-session contention, say how many writers / worktrees / remotes you used — without pasting credentials.

## Docs on this site

Human-facing Pages live under `docs/`. Prefer short product language over dumping design specs. Lead with the multi-session contention motive when it helps; do not turn Pages into an agent-governance ledger. Design specs and verification matrices stay in `.spec/` and `verify/` for maintainers.

> [!NOTE]
> Use **Edit this page** in the page-actions menu (when present) to open the Markdown source on GitHub.

## For maintainers and agents

Start from [`AGENTS.md`](https://github.com/modootoday/arks/blob/main/AGENTS.md). Clean-room Docker runs prove claims; they are not the end-user install path. See [Performance](../performance/) and [Concurrent sessions](../concurrent-sessions/) for how verification framing is explained publicly.

Release-channel ops (OIDC Environments, tag dry-run, draft Releases): [Release channels](../release/). Dry-run readiness is not a registry publish PASS.

Secrets-gated live forge smoke / scale harness: [Forge scale harness](../forge-scale/). Dry-run readiness is not a live latency PASS.

Partial-clone `--filter=blob:none` / `tree:0` / related FS4 filters + G6 `object:type` / `combine:` + `sparse:oid=<hex>` + promisor are claimed on pack transports; other unlisted exotic specs remain refuse-with-reason — see [Limits](../limits/) and [Git vs arks](../git-vs-arks/). Shallow `--depth` remains claimed. In-process Git LFS / `arks forge` / `arks host-env` are **trimmed** over-design (use `git-lfs` / `gh` / host tooling); `scalar` stays refuse.

SSH, OpenPGP, and X.509 commit signing plus `tag -s` / `verify-tag` are claimed (SSH **G37**; OpenPGP/X.509/`tag -s` **G38**). Forge signature UX remains Impossible — see [Limits](../limits/) and [Git vs arks](../git-vs-arks/).

Honest feature comparison: [Git vs arks](../git-vs-arks/). Full Symmetry **FS_final** is Done for git feature symmetry DoD; operator summary stays **PARTIAL** — see [Completeness](../completion/). Remaining non-goals are documented Impossible / trimmed / owner-policy (**L9**, live G8 latency without secrets, crypto immunity, forge/host-env/LFS product CLI). Clean-room Docker e2e (L8 / S6) is maintainer verification — not the end-user install path.

## Next

[Getting started](../getting-started/) [Git vs arks](../git-vs-arks/) [Release channels](../release/) [Forge scale harness](../forge-scale/) [Limits](../limits/) [Source on GitHub](https://github.com/modootoday/arks)
