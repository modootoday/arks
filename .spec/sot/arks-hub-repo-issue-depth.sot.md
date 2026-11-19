---
id: sot:root/arks-hub-repo-issue-depth
title: arks forge client — repo and issue depth (C8)
status: active
created: 2026-11-19T21:00:00Z
updated: 2026-11-19T21:00:00Z
tags: [arks-hub, gh-symmetry, repo, issue, clone, sync, contents, gitignore, license, autolink, deploy-key, issue-artifact, issue-develop, graphql, partial, host-advisory, native-api]
verify: manual
rel:
  depends_on:
    - sot:root/arks-hub-product-intent
    - sot:root/arks-hub-cli-surface
    - sot:root/arks-hub-gh-command-matrix
    - sot:root/arks-self-host-server
    - sot:root/arks-remote
    - sot:root/crate-architecture
    - sot:root/product-verification-clean-room
  evidence: []
  governs:
    - crates/arks-hub/src/client
    - crates/arks-hub/src/forge_catalog.rs
    - crates/arks-hub/src/forge_git.rs
    - crates/arks-hub/src/forge_repo_issue.rs
    - crates/arks-hub/src/store_repo_issue.rs
---

# arks forge client — repo and issue depth (C8)

Version **0.1.0**. Normative behaviour for the `repo` and `issue` rows that were `Deferred` in
`verify/matrices/hub-gh-arks-e2e-matrix.csv` (cloud drop-in rung **C8**, owner priority repo + issue).
Client leaves live in the internal `arks-hub` crate (flat `arks repo …` / `arks issue …`, transitional
`arks hub …`); the local forge equivalents live in the `arks serve` forge module so every row can be
exercised end to end without GitHub. Never spawns `git` or `gh` (rule 30); Git work runs in-process
through `arks-remote` / `arks-repo`.

**Peer pin:** `gh` 2.103.3 ([[sot:root/arks-hub-gh-command-matrix]]). Every leaf below has a manual page
at the pin, so none is Refuse/N/A for lack of primary documentation.

**Status vocabulary:** rows proven only by host runs of the local-forge e2e are **Partial (host
advisory)** — never Claimed without a Docker clean-room receipt ([[sot:root/product-verification-clean-room]],
rule 28).

## Rule

### Primary sources (per row)

| gh leaf | Manual | Wire |
|---------|--------|------|
| `repo clone` | <https://cli.github.com/manual/gh_repo_clone> | `GET /repos/{o}/{r}` (`clone_url`, `fork`, `parent`) — <https://docs.github.com/en/rest/repos/repos#get-a-repository>; Git transport via `arks-remote` |
| `repo sync` | <https://cli.github.com/manual/gh_repo_sync> | Local mode: in-process fetch + fast-forward ([git-merge §FAST-FORWARD](https://git-scm.com/docs/git-merge#_fast_forward_merge)) |
| `repo read-dir` | <https://cli.github.com/manual/gh_repo_read-dir> | `GET /repos/{o}/{r}/contents/{path}?ref=` — <https://docs.github.com/en/rest/repos/contents#get-repository-content> |
| `repo read-file` | <https://cli.github.com/manual/gh_repo_read-file> | same contents endpoint (base64 `content`) |
| `repo gitignore list\|view` | <https://cli.github.com/manual/gh_repo_gitignore_list>, `…_view` | `GET /gitignore/templates[/{name}]` — <https://docs.github.com/en/rest/gitignore/gitignore> |
| `repo license list\|view` | <https://cli.github.com/manual/gh_repo_license_list>, `…_view` | `GET /licenses[/{license}]` — <https://docs.github.com/en/rest/licenses/licenses> |
| `repo autolink create\|delete\|list\|view` | <https://cli.github.com/manual/gh_repo_autolink> | `GET\|POST /repos/{o}/{r}/autolinks`, `GET\|DELETE …/autolinks/{id}` — <https://docs.github.com/en/rest/repos/autolinks> |
| `repo deploy-key add\|delete\|list` | <https://cli.github.com/manual/gh_repo_deploy-key> | `GET\|POST /repos/{o}/{r}/keys`, `DELETE …/keys/{id}` — <https://docs.github.com/en/rest/deploy-keys/deploy-keys> |
| `issue artifact create\|delete\|download\|edit\|list\|view` | <https://cli.github.com/manual/gh_issue_artifact> (preview) | `GET\|POST /repos/{o}/{r}/issues/{n}/artifacts`, `GET\|PATCH\|DELETE …/artifacts/{k}` — <https://docs.github.com/en/rest/issues/artifacts> |
| `issue develop` | <https://cli.github.com/manual/gh_issue_develop> | GraphQL `Issue.linkedBranches`, `createLinkedBranch(input: {issueId, oid, name, repositoryId})` — <https://docs.github.com/en/graphql/reference/issues> |

### Client behaviour (normative)

1. **Output shape.** Where the manual documents output (raw template/license body, raw file bytes,
   `--json <fields>` with the listed field names, `<number>-<name>.<ext>` download names, `.url`
   Internet Shortcut files), arks matches it. Where the manual is silent (list tables, success lines),
   arks prints plain tab-separated rows and one-line confirmations; these are not claimed byte-identical
   with `gh`.
2. **`--jq` / `--template` / `--web`** refuse with exit 2 (no jq engine, no Go templates, no browser
   launch). `repo license view --web` prints the URL instead of launching a browser.
3. **No prompts.** Leaves whose gh peer prompts (`repo autolink delete`, `issue artifact delete`)
   require `--yes`; without it they refuse with exit 2.
4. **`repo clone`.** `OWNER/REPO` (or `REPO` → authenticated user from `GET /user`) resolves
   `clone_url` from the API; a URL argument is cloned directly. Transport dispatch matches `arks clone`
   (http(s), ssh, `git://`, local path). For forks, the parent's `clone_url` is added as remote
   `upstream` (`-u/--upstream-remote-name`, `@owner`, `--no-upstream`). Only `--depth N` passes through
   after `--`; other git-clone flags refuse. gh's `gh-resolved` remote config is not written.
5. **`repo sync` (local mode).** Source remote is `upstream` when configured, else `origin`; branch is
   `--branch` or the remote's default (HEAD) branch. The checked-out branch must be clean for tracked
   paths (`git status --untracked-files=no` sense); a non-fast-forward refuses unless `--force`, which
   hard-resets the branch (and the work tree when it is checked out). Fetch runs in-process and is
   available for http(s), ssh and `git://` remotes; local-path remotes refuse.
   **Remote mode** (`sync <destination-repository>`) and `--source` refuse with exit 2: arks has no
   fork model (`repo fork` is Refuse/N/A).
6. **`repo read-dir` JSON.** `gitSHA`, `gitType`, `name`, `nameRaw`, `path`, `pathRaw`, `size`,
   `submodule`, `type` derive from the contents API; `mode` / `modeOctal` refuse (the contents API does
   not return a file mode). `read-file` refuses content with terminal escape sequences on stdout unless
   `--allow-escape-sequences`; `--output` writes raw bytes and refuses an existing path without
   `--clobber`. No pager is spawned.
7. **`issue artifact`.** Files become `generic` (or `embedded_html` for `.html`/`.htm`) documents
   named after the file (`<file>#<name>` overrides; parentheses stripped); URLs and `.url` files become
   `link` artifacts named after the host; `--body` / `--body-file` (`-` = stdin) create one artifact.
   `view` / `download --version N` and `edit --restore-version N` use the version history from
   `GET …/artifacts/{k}`. `--attach` refuses (no documented upload endpoint).
8. **`issue develop`.** `--list` prints linked branch names; create resolves the issue/repository
   node ids and the base commit (`--base` remote branch, else the branch repository's default branch),
   then calls `createLinkedBranch` (`--name`, `--branch-repo`). `--checkout` fetches `origin`
   in-process and switches only when the work tree is clean. `--worktree` refuses (Partial).

### Local forge equivalents (normative for `arks serve`)

9. **Hosted git storage.** A forge record `owner/name` is backed by the bare repository
   `<forge-data>/git/<owner>/<name>.git` when present. `GET /repos/{o}/{r}` then adds `clone_url`
   (`$ARKS_HUB_CLONE_BASE/<owner>/<name>.git` when that variable is set on the forge host, else the bare
   repository path), `default_branch` and `fork: false`. This layout is a local-forge convention, not a
   GitHub contract.
10. **Catalogs** are a small arks-authored set (`.gitignore`: Node, Python, Rust; licenses: ISC, MIT
    with `[year]` / `[fullname]` placeholders), not GitHub's catalog.
11. **GraphQL.** `POST /graphql` serves only the operation names the arks client sends
    (`IssueLinkedBranches`, `IssueDevelopContext`, `CreateLinkedBranch`), keyed by `operationName` and
    `variables`; any other operation returns HTTP 400. It is not a general GraphQL server.
    `createLinkedBranch` creates `refs/heads/<name>` in the hosted bare repository and refuses an
    existing branch; an omitted name defaults to `<number>-<slugged title>`.
12. Autolink, deploy-key and artifact records persist in the forge `db.json`; validation errors return
    HTTP 422 as in the REST reference (`url_template` must contain `<num>`; deploy keys must be an
    OpenSSH public key line).

### Row dispositions (target)

| Rows | Target status | Reason |
|------|---------------|--------|
| `repo gitignore` (+list, view), `repo license` (+list, view), `repo autolink` (+create, delete, list, view), `repo deploy-key` (+add, delete, list), `repo read-dir`, `repo read-file`, `repo clone`, `issue artifact` (+create, delete, download, edit, list, view) | **Partial (host advisory)** | Implemented + local-forge e2e on host; no Docker receipt yet |
| `repo sync` | **Partial (host advisory)** | Local mode only; remote mode / `--source` refuse |
| `issue develop` | **Partial (host advisory)** | `--list` / create / `--checkout`; `--worktree` refuses; GraphQL proven only against the local forge |

None of these rows becomes Claimed without a Docker clean-room run of the harness block
`ARKS_HUB_IW_REPO_ISSUE=1` (`verify/scripts/run-hub-gh-arks-matrix-e2e.sh`).

## Why

The C8 rung asks for repo and issue first. Every Deferred row has a primary manual page at the pin, so
the honest move is to implement them against documented REST/GraphQL shapes, keep the local forge able
to answer them, and label host-only proof as advisory. Refusing remote sync and `--worktree` keeps the
surface fail-closed where arks has no matching engine feature.

## Verification

1. `cargo test -p arks-hub` (unit tests for catalogs, timestamps, argv parsing, base64, artifact names).
2. Host run: `ARKS_HUB_IW_REPO_ISSUE=1 verify/scripts/run-hub-gh-arks-matrix-e2e.sh` writes host-advisory
   outputs under `.spec/evidence/<timestamp>-hub-repo-issue/`.
3. Docker clean-room run of the same block is required before any row moves to Claimed.
4. Product paths contain no `Command::new("git"|"gh")`.
