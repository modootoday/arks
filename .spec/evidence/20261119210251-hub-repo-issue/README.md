---
id: evidence:root/20261119210251-hub-repo-issue
title: C8 repo and issue rows — local-forge e2e (host advisory)
created: 2026-11-19T21:02:51Z
tags: [hub, gh-symmetry, repo, issue, c8, host-advisory, local-forge]
rel:
  depends_on:
    - sot:root/arks-hub-repo-issue-depth
    - sot:root/arks-hub-gh-command-matrix
---

# C8 repo and issue rows — local-forge e2e (host advisory)

**Class: HOST ADVISORY.** Not a Docker clean-room receipt; MUST NOT be cited as Claimed or PASS
(rules 28 / 60). The sandbox had no Docker daemon.

Entry: `ARKS_HUB_EVIDENCE_HOST=1 ARKS_HUB_IW_REPO_ISSUE=1 ARKS_BIN=./target/release/arks
verify/scripts/run-hub-gh-arks-matrix-e2e.sh` at tip `895fecc` (Linux 6.18, release build).
The harness runs one `arks serve` (smart HTTP git + forge API, no SSH), seeds the hosted bare
repository `<forge-data>/git/local/hubri.git` by `arks push` over HTTP, and drives every C8 row with
the same `arks` binary. No `git` or `gh` executable is used. Full transcript: `run.log`; per-step
outputs: `*.out` (temp prefix scrubbed to `$WORK`); summary: `receipt.json`.

## Result

| Area | Steps green | Notes |
|------|-------------|-------|
| `repo read-dir` / `read-file` | 7 | listing, `--json` fields, raw bytes, `--output`; `--json mode` and no-clobber refuse |
| `repo gitignore` / `license` | 5 | list + view; case-sensitive template miss refuses |
| `repo autolink` | 7 | create/list/view/delete; template without `<num>` and delete without `--yes` refuse |
| `repo deploy-key` | 4 | add/list `--json`/delete |
| `repo clone` | 3 | API `clone_url` → in-process smart HTTP clone; non-empty dest and `-- --bare` refuse |
| `repo sync` | 5 | fast-forward; dirty tree refuse; diverged refuse; `--force` matches source HEAD and drops the local-only file; remote mode refuses |
| `issue artifact` | 16 | issue create; artifact create (file, URL, `--body`), list `--type`, edit, view `--json versions`, `--version 1`, download names + `.url`, `--output -`, `--restore-version`, delete; `--attach`, existing file and missing `--yes` refuse |
| `issue develop` | 6 | `--name/--base`, default name, `--list`, `--checkout` switches the clone; duplicate branch and `--worktree` refuse |

56 green steps (including 3 setup steps: login, repo create, repo view), 0 failures (`run.log`).

## Honesty

- Every row stays **Partial (host advisory)** in `verify/matrices/hub-gh-arks-e2e-matrix.csv` until the
  same block runs in the Docker clean-room venue.
- Proven only against the local forge; no live GitHub call was made (GraphQL in particular is proven
  only against the local forge's three named operations).
- Default list/success lines are plain text; the gh manual does not document them, so they are not
  claimed byte-identical with `gh`.
