---
id: sot:root/arks-repo
title: arks-repo unit
status: active
created: 2026-11-08T10:05:00Z
updated: 2026-11-09T02:30:00Z
tags: [m3, unit, repo, ring-4, worktree, commondir]
verify: manual
rel:
  governs: [crates/arks-repo]
  depends_on:
    - sot:root/program-roadmap
    - plan:root/20261108090648
    - plan:root/20261109022956
    - sot:root/git-compat-surface
    - sot:root/core-object-model
    - sot:root/crate-architecture
    - sot:root/arks-store
    - sot:root/arks-refs
    - sot:root/arks-object
    - sot:root/arks-oid
    - research:root/20261108071914
  evidence: [evidence:root/20261108191033]
---

# arks-repo unit

Version **0.2.0**. Unit SoT for Ring 4 repository **discovery/layout** and **composition** of object store + refs ([[plan:root/20261108090648]], M3 Phase R under [[sot:root/program-roadmap]] **v0.3.1**). Layout rules from [gitrepository-layout](https://git-scm.com/docs/gitrepository-layout) and [[sot:root/git-compat-surface]] § same-`.git` layout. Dependency direction: [[sot:root/crate-architecture]] (`arks-repo` → `arks-store`, `arks-refs`, `arks-object`, `arks-oid`).

**Residual (post-M9):** linked worktree / `commondir` / `$GIT_COMMON_DIR` claimed for [[plan:root/20261109022956]] `worktree` surface.

## Rule

1. **Role:** discover a Git repository root / `GIT_DIR` (and common-dir / worktree splits documented in gitrepository-layout); expose composed access to:
   - object persistence via [[sot:root/arks-store]] (`ObjectStore` / filesystem Git backend) under **`$GIT_COMMON_DIR/objects`**;
   - refs via [[sot:root/arks-refs]] (`FilesystemRefStore`) with dual-root when linked (per-worktree `HEAD` vs shared `refs/`);
   - optional **index path** access (path to `$GIT_DIR/index` for callers of [[sot:root/arks-index]]) — repo does **not** re-implement index codecs;
   - **work-tree path** when discoverable (main `.git/` parent or linked worktree via `gitdir` back-pointer).
2. **Public surface:** `discover_git_dir`, `parse_gitdir_file`, `parse_commondir`, `GitDirKind`, `RepoError`, `Repository::{discover, open, git_dir, common_dir, work_tree, kind, index_path, store, store_mut, refs, refs_mut, head_oid, head_target, get_object}`.
3. **Discovery MUST** honor environment / path conventions documented by Git for locating `.git` or bare roots, plus `commondir` / `$GIT_COMMON_DIR` when present (cite gitrepository-layout / [git-worktree(1)](https://git-scm.com/docs/git-worktree); do not invent a parallel proprietary layout). In Git-compat mode the on-disk tree remains stock-Git layout ([[sot:root/git-compat-surface]]).
4. **Composition MUST NOT** bypass L0 Git locks owned by refs/index crates; MUST NOT activate CELIR ([[sot:root/storage-celir-boundary]]); MUST NOT introduce L1+ lease APIs ([[sot:root/lease-concurrency-intent]]).
5. **Zero `gix`** and no product-path `git` subprocess ([[adr:root/20261108083507]]).
6. **Out of scope:** pack/protocol; CLI porcelain orchestration (owned by `arks`); reftable; packed-refs rewrite; CELIR backends; product leases; inventing object or ref semantics already owned by inner crates.

## Why

`arks-repo` is the Ring 4 assembly point so higher layers and clean-room harnesses open one layout handle instead of wiring store + refs ad hoc, while keeping format ownership in inner unit crates.

## Verification

1. Host (advisory): `CARGO_BUILD_JOBS=2 cargo test -p arks-repo` when the crate is a workspace member.
2. Clean-room **G0** includes repo unit tests; **G2** exercises refs through a discovered `GIT_DIR` (`verify/matrices/matrix-m3.md`); carrying receipt: [[evidence:root/20261108191033]].
3. `spec check` clean for this SoT.
4. Workspace `cargo metadata` shows no `gix` edge on `arks-repo`; edges only inward to store/refs/object/oid as claimed.
