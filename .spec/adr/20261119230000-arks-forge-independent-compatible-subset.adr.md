---
id: adr:root/20261119230000
title: arks forge is independent; GitHub-shaped wire only for arks-owned primitives
status: active
created: 2026-11-19T23:00:00Z
updated: 2026-11-19T23:50:00Z
tags: [arks-hub, forge, arks-serve, independence, gh-symmetry, scope, c8, decision]
rel:
  depends_on:
    - sot:root/product-intent
    - sot:root/remote-host-compatibility-surface
    - sot:root/arks-hub-product-intent
    - sot:root/arks-hub-gh-command-matrix
    - sot:root/arks-self-host-server
    - plan:root/20261119210000
---

# arks forge is independent; GitHub-shaped wire only for arks-owned primitives

## Context

arks is a separate, independent service modelled on three peers: `git` (porcelain and plumbing),
`gh` (forge client command names), and `git daemon` (serving). It is not a port of GitHub.

The canon disagreed with itself. [[sot:root/remote-host-compatibility-surface]] says "arks is not a
GitHub-compatible forge", while [[sot:root/arks-hub-product-intent]] named a "GitHub-compatible API"
as the first verify peer, [[sot:root/arks-hub-gh-command-matrix]] put Actions `run` / `workflow`
in phase v1, and [[sot:root/arks-self-host-server]] described a "GitHub-class self-host". Work under
rung C8 of [[plan:root/20261119210000]] followed the second reading and started porting GitHub
platform features into the forge. That work was reverted.

## Decision

1. **Independent forge.** `arks serve` hosts the arks forge. arks defines the forge primitives and
   their semantics: repositories, issues, pull requests, reviews, labels, milestones, releases with
   assets, and authentication. A primitive enters scope because arks needs it as a forge, never
   because GitHub has it.
2. **Compatible subset (wire convention).** For arks-owned primitives, the forge API uses
   GitHub-shaped REST paths and payload field names (the shape the local forge already serves). The
   result: the arks client can also talk to a GitHub-class host through `--host`, and gh-shaped
   tooling can reach `arks serve` for that subset. Compatibility is a convention, not a scope driver.
   A GitHub endpoint or field is adopted only when it serves an arks-owned primitive. GraphQL is not
   required.
3. **Scope test: not GitHub-exclusive.** arks mirrors `git`, `gh` and `git daemon` symmetrically
   and must be hostable on its own. A `gh` command is in scope when its function can run on an
   independent host (it is not a GitHub-exclusive service); arks keeps it symmetric with `gh`.
   A command is out of scope when it only exists as a GitHub platform service. Out of scope:
   Actions and its CI runtime (`run`, `workflow`, `cache`, Actions `secret` / `variable`),
   Codespaces, Copilot, artifact attestations (`attestation`, `release verify*`), extensions,
   agent tasks and skills. Each needs its own SoT amendment before it can come back.
4. **Verify peers.** The first verify peer for the forge client is the local `arks serve` forge.
   GitHub.com is an optional interop peer for the compatible subset only. It stays secrets-gated
   (D2 of [[adr:root/20261119180100]]), and missing GitHub receipts never block an arks rung.
5. **Muscle memory is naming only.** gh supplies command and flag names, so users keep their habits.
   It does not supply product scope.
6. **In scope by the test (owner decision, 2026-11-19).** `discussion`, `gist`, `project`,
   `repo autolink`, `repo deploy-key`, `ssh-key`, `gpg-key`, rulesets (`ruleset` / `rs`, branch
   protection), template catalogs (`repo gitignore`, `repo license`) and commit status
   (`pr checks`, against an arks-owned status primitive rather than GitHub Apps check runs) are in
   scope. They keep phase `later` until implemented.
7. **Build directive (owner, 2026-11-19).** `discussion`, `gist`, `project`, `repo autolink`,
   `repo deploy-key`, `ssh-key` and `gpg-key` are built as arks-owned forge features: arks defines
   their storage and semantics in the `arks serve` forge, and the client keeps `gh`-symmetric argv
   and flags so the same commands work against `arks serve`. Each row moves to phase `v1` only when
   its leaves are implemented and tested against the local forge.

## Consequences

- [[sot:root/arks-hub-product-intent]], [[sot:root/arks-hub-gh-command-matrix]] (with its CSV),
  [[sot:root/arks-self-host-server]] and [[sot:root/remote-host-compatibility-surface]] are amended
  to this reading.
- The CSV moves `run*`, `workflow*`, `cache*`, `secret*`, `variable*` and `release verify*` to
  `out-of-scope`. `ruleset*` / `rs`, `repo gitignore*` / `repo license*` and `pr checks` are
  `later` (in scope, not yet implemented).
- The existing `run` / `workflow` client leaves in `arks-hub` are removed in a separate
  implementation commit. They were never Claimed.
- Rung C8 is redefined as the daemon flag surface plus arks-owned forge primitives (`pr checkout`,
  release assets) end to end against the local `arks serve` forge.
