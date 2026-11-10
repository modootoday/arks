---
id: research:root/20261111041000
title: Public history clean rewrite via delete+recreate remote
status: active
created: 2026-11-10T19:10:00Z
updated: 2026-11-10T19:10:00Z
tags: [public-history, rewrite, delete-recreate, arks, arks-hub]
verify: host
rel:
  depends_on:
    - research:root/20261111035700
    - sot:root/product-intent
    - sot:root/arks-hub-product-intent
  evidence: []
---

# Public history clean rewrite (delete + recreate remote)

**Canon:** `C:\workspaces\gir` → `https://github.com/modootoday/arks`  
**Owner intent:** No real public release yet. Soft feature-delete is not enough — **erase the public commit story** by deleting the GitHub remote and recreating it empty, then pushing only after local cleanup + review.

## Status (updated this round)

| Step | State |
|------|--------|
| Assess old `main` | Done — tip was `6a15b68`, ~158 commits, pack ~4.6 MiB; **public**; 0 stars / 0 forks / 0 open Issues·PRs; Pages on; ~295 Actions runs (gone with delete) |
| Over-design trim | **Incomplete** — tip had partial trim (`584b0ef`); WIP still dirty (credential no-`git`/`gh` spawn, help matrix, untracked `arks-hub` SoT/skeleton) |
| Remote delete | **Done** — `modootoday/arks` deleted |
| Remote recreate | **Done** — empty repo recreated **public** (matched previous visibility) |
| Local `origin` retarget | **Done** — still `https://github.com/modootoday/arks.git` |
| Push clean history | **Deferred** — **do not push** until owner says after local cleanup + re-review |
| Force-push rewrite on old remote | **Not used** — replaced by delete+recreate |

**Gate phrase for later push (not this round):** owner must explicitly approve after history re-review / public-surface check (e.g. “push the clean history now”).

## Product end-state (what the clean tree must reflect)

1. **Single published CLI = `arks` only** — git CLI / protocol / same-`.git` + race/lease + measured perf + server paths. Over-design trimmed (`forge`, `host-env`, in-process LFS product, `info/lfs`, CELIR product Claimed path) at tip ≥ `9df7fb9`.
2. **`arks daemon`** ↔ `git-daemon` via internal crate `arks-daemon` (not a separate release).
3. **`arks hub …`** ↔ `gh` via internal crate `arks-hub` (design SoT/matrix; native host API; **not** a separate release binary).
4. **Absolute runtime:** product `arks` **never** subprocess `git` or `gh`. (Operator `gh` for repo admin is unrelated and allowed.)

## Recommended clean history shape (local, before first push)

Choose when trim + hub SoT + git-CLI WIP are committed and reviewed:

### Option A — single squashed “Initial public release” (default if still pre-release)

One orphan commit from the finished tree. Public story = one line.

### Option B — small curated series (if narrative helps reviewers)

Example only (3–5 commits, no AI thrash):

1. SoT + docs + licensing + product intent (arks + arks-hub design)
2. Core workspace / engine crates (git symmetry)
3. CLI + remote/server + daemon
4. Verify harness + CI
5. (optional) arks-hub stub if ready

**Not recommended:** `git filter-repo` on the old 158-commit graph — delete+recreate already drops public history; orphan/squash is enough.

## What stays in tree (after trim finishes)

| Keep | Drop / move |
|------|-------------|
| `crates/arks*` git engine + CLI + lease + server | Product forge / host-env / in-process LFS surfaces (already refuse/trim path) |
| `arks-hostfs` non-default in-tree only | Claimed product path for hostfs |
| Design SoT under `.spec/sot/` (incl. arks-hub) | AI iteration noise in commit messages (via rewrite) |
| `verify/`, maintainer `.spec/`, honest docs | Marketing Claimed language for trimmed rows |
| CI workflows needed for first push | Secrets must be re-added after recreate |

Fix before first push if still wrong: `workspace.package.repository` previously pointed at `humanymous/gir` — should match `modootoday/arks`.

## Risks / what the delete already broke

| Item | Impact |
|------|--------|
| Old clone URLs | Same URL path returns **new empty** repo; old SHAs unreachable on GitHub |
| Anyone who cloned | Local clones still have old history; they must re-clone or retarget after first push |
| Open PRs / Issues | Were 0; recreate starts empty |
| Stars / forks | Were 0 |
| Actions run history | **Gone** (~295 runs) |
| GitHub Pages | **Gone** until re-enabled from new `main` `/docs` after push |
| Repo secrets / environments | **Gone** — reconfigure before release workflows work |
| Webhooks | Were none |
| Local backup | Keep `backup/pre-rewrite-0b2301e` and/or a local tag of old tip for forensics; do not publish that history |

## Checklist — operator order

### Done

- [x] Confirm `origin` → `modootoday/arks`
- [x] Detect visibility → **public**
- [x] `gh repo delete modootoday/arks --yes`
- [x] `gh repo create modootoday/arks --public …` (empty; no `--source` push)
- [x] `git remote set-url origin https://github.com/modootoday/arks.git`
- [x] Confirm `git ls-remote origin` empty / no branches
- [x] **No push**

### Remaining (owner / later agents) — push deferred

- [ ] Finish over-design trim + no `git`/`gh` subprocess on product paths
- [ ] Finish arks-hub design SoT / matrix (+ optional stub)
- [ ] Finish git-CLI symmetry WIP; commit on normal working branch as needed
- [ ] Build local clean history (`public-clean` orphan **or** single initial commit) from finished tree only
- [ ] Re-review history messages + tree (no thrash, no secrets, repository URL fixed)
- [ ] Full public-surface check (README/docs Claimed honesty vs trim)
- [ ] Owner explicit push OK
- [ ] `git push -u origin <clean-branch>:main` (first content)
- [ ] Re-enable Pages / secrets / branch protection as needed

## Exact commands (reference)

### Already run (this round)

```powershell
gh repo delete modootoday/arks --yes
gh repo create modootoday/arks --public --description "arks — Git-compatible revision CLI (Agent Revision Kinetic System)" --disable-wiki
git -C C:\workspaces\gir remote set-url origin https://github.com/modootoday/arks.git
git -C C:\workspaces\gir ls-remote origin   # expect empty
```

### Later — local orphan clean branch (after tree ready; do not push until approved)

```powershell
cd C:\workspaces\gir
git branch backup/pre-public-clean HEAD   # optional safety tip
git checkout --orphan public-clean
git rm -rf --cached . 2>$null
# ensure working tree is the finished clean snapshot (trim+hub SoT done)
git add -A
# exclude junk: .cargo-target/, target*/, tmp-*
git reset HEAD -- .cargo-target target-pilot target tmp-gh-cmds.txt tmp-gh-reference.txt 2>$null
git commit -m "Initial public release of arks (git-symmetric CLI)."
# STOP — wait for owner push approval
# git push -u origin public-clean:main
```

### Option B sketch (curated)

Same orphan start, then interactive-style sequential commits from path slices (SoT → crates → verify) without importing old SHAs.

## English note

Remote was **deleted and recreated empty** as `modootoday/arks` (**public**, matching prior visibility). Local `origin` points at the new repo. **Push is deferred** until trim, arks-hub SoT/matrix, git-CLI symmetry, history re-review, and public-surface check complete — then only on explicit owner approval.
