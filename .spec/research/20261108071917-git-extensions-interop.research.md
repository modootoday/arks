---
id: research:root/20261108071917
title: Git repository extensions and interop
status: active
created: 2026-11-08T07:19:17Z
updated: 2026-11-08T07:25:00Z
tags: [git, extensions, shallow, partial-clone, bundle, alternates, sha256]
rel:
  depends_on: [research:root/20261108071859]
---

# Git repository extensions and interop

## Question

Which documented repository extensions and interop features change the compatibility surface (must honor, must refuse, or optional accelerate)?

## Method

Primary: repository-version, git-config `extensions.*`, hash-function-transition, partial-clone, shallow, gitrepository-layout (alternates, replace, shallow file), gitformat-bundle, protocol capability pages.

## Findings

### Repository format versions — repository-version

- **URL:** https://git-scm.com/docs/repository-version
- `core.repositoryformatversion` = `0` (classic) or `1` (extensions namespace enforced).
- Version 1: implementations **MUST** read `extensions.*`; unknown extension key or unknown value → **MUST NOT proceed** (data-loss avoidance).
- Prefer per-file version bumps and ignore-safe optional files over whole-repo version bumps.
- Wire access: older clients may still talk to a v1 repo **through a server** that understands v1.

### Hash algorithms — hash-function-transition + extensions.objectFormat

- **URL:** https://git-scm.com/docs/hash-function-transition
- SHA-256 repositories via repository format extension; object names and embedded OIDs use SHA-256; older Git cannot read them.
- Config: `extensions.objectFormat`, optional `extensions.compatObjectFormat` (git-config) for compatibility mapping (see also gitformat-loose object-map).
- Bundle v3 `object-format` capability must match (gitformat-bundle).
- **Compat impact:** treat SHA-1 and SHA-256 as distinct repository worlds unless compat mapping is implemented per docs.

### Partial clone — partial-clone + filter capability

- **URL:** https://git-scm.com/docs/partial-clone
- Pack protocol `filter` capability; filtered packs marked promisor; `extensions.partialClone` prevents unaware Git from operating unsafely.
- Independent of shallow/DAG limiting (non-goal: conflict with shallow).
- **Compat impact:** GIR servers advertising `filter` must omit objects per filter-spec; clients must handle promisor fetches.

### Shallow clones

- Protocol: deepen / shallow / unshallow / deepen-since / deepen-not in gitprotocol-capabilities / pack / v2.
- On-disk: `$GIT_DIR/shallow` per gitrepository-layout; design note https://git-scm.com/docs/shallow
- **Compat impact:** incorrect shallow boundaries produce wrong histories; honor advertised shallow lines.

### Alternates

- `objects/info/alternates` and `http-alternates` in gitrepository-layout; env `$GIT_ALTERNATE_OBJECT_DIRECTORIES`.
- Incomplete ODB; unsuitable for dumb publish unless carefully relative.
- Clone `--shared` / `--reference` set alternates (git-clone).

### Replace refs vs grafts

- `refs/replace/` exchangeable; `info/grafts` legacy and problematic for transfer (layout doc).
- Prefer replace for GIR; treat grafts as read-legacy only if implemented.

### Bundles as interop containers

- gitformat-bundle v2/v3; unknown v3 capability aborts.
- Related: bundle-uri for bootstrap.

### Sparse checkout / sparse index

- https://git-scm.com/docs/sparse-checkout
- https://git-scm.com/docs/sparse-index
- Patterns in `info/sparse-checkout`; index may store directory stubs.
- Optional working-tree feature; still must not corrupt full checkouts.

### Ref storage extension

- `extensions.refStorage=reftable` selects reftable backend (see [[research:root/20261108071914]]).

### Other extensions (claim names in git-config)

- Authoritative list: https://git-scm.com/docs/git-config under `extensions.*`
- Examples encountered in research: `objectFormat`, `compatObjectFormat`, `partialClone`, `refStorage`, `worktreeConfig`, `preciousObjects`, …
- **Rule for GIR:** never set an extension GIR does not implement; never open a v1 repo with unknown extensions.

### Security-relevant documented behaviors

- Hash transition (collision resistance motivation) — hash-function-transition.
- Transfer MUST validate pkt-line / HTTP status / Content-Type (gitprotocol-http).
- Signed objects/pushes — gitformat-signature; verification policy is product, format is Git’s.
- Hooks can enforce receive rules — githooks; not a wire format but affects push interop.

## Open follow-ups

- Exhaustive `extensions.*` key table with GIR support status columns.
- Promisor remote protocol edge cases / large-object-promisors doc.
