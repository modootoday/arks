---
id: research:root/20261108073650
title: Product verification matrix
status: active
created: 2026-11-08T07:36:50Z
updated: 2026-11-08T18:30:00Z
tags: [verification, docker, clean-room, matrix, git-compat, github, journey]
rel:
  depends_on: [sot:root/git-compat-surface, sot:root/remote-host-compatibility-surface, sot:root/clean-room-journey-compare]
---

# Product verification matrix

## Question

Which verification dimensions does GIR need, and which MUST run in the Docker clean-room (authoritative) versus MAY run on the host (advisory / non-authoritative)?

## Method

1. Read normative intent in [[sot:root/product-verification-clean-room]], [[sot:root/git-compat-surface]], and [[sot:root/remote-host-compatibility-surface]].
2. Cite humanymous completion-authority pattern (`60-e2e-docker-only.md`: host e2e not “done”; Docker compose is the operator/CI path) without copying humanymous lab topology into GIR.
3. Partition gates by product concern: unit speed, Git format/protocol fixtures, GitHub-class remote smoke, and deferred lease/concurrency.
4. Mark each cell **clean-room required** vs **host advisory**; note open infra decisions (image, Git pin, compose path) owned by the clean-room SoT.
5. Do not invent Git or GitHub wire semantics here — only where tests run.

Research snapshot date: **2026-11-08**.

## Findings

### Authority summary

| Venue | Role | May claim “verified”? |
|-------|------|------------------------|
| Docker clean-room (pinned image + deterministic entrypoint) | **Authoritative** | Yes, when the gate recipe passes |
| CI using the same images/compose | **Authoritative** (same as clean-room) | Yes |
| Host PATH / laptop toolchains | **Advisory** | No (unless mirrored green in clean-room) |

### Matrix

| Dimension | What it proves | Clean-room | Host | Notes / owning SoT |
|-----------|----------------|------------|------|--------------------|
| **Unit / crate tests** | Local logic, parsers, pure helpers | **Required** for any release aggregate that cites them as verified; otherwise optional mirror | **Advisory** OK for edit-loop speed | Fast feedback; host green alone ≠ release gate |
| **Lint / fmt / static analysis** | Style and trivial correctness | Optional mirror in CI image | **Advisory** typical | Not a substitute for interop gates |
| **Git-compat fixtures** | Round-trip with **stock Git** for claimed subset (objects, index, refs, pack, protocols) | **Required** — stock Git is the in-image pin | Advisory prototyping only | [[sot:root/git-compat-surface]]; refuse tests (unknown extensions, etc.) included |
| **Protocol / pkt-line fixtures** | Offline wire traces vs documented behavior | **Required** when claimed | Advisory | [[research:root/20261108071916]] semantics; execution venue = clean-room |
| **GitHub-class remote smoke** | Live or CI clone/fetch/push to github.com-class remotes | **Required** for verified remote-client claims | Forbidden as sole proof | [[sot:root/remote-host-compatibility-surface]]; secrets via env/CI, never tree |
| **Recorded remote regression** | Redacted pkt-line/HTTP fixtures replay | **Required** for offline remote regression claims | Advisory authoring | Prefer clean-room replay as CI default when live creds absent |
| **Partial/shallow / filter matrix** | Negotiated filters against host or fixture | **Required** when feature claimed | Advisory | Only for claimed subset; cite host docs via remote research |
| **Lease / concurrency product gates** | Future product behavior beyond Git’s documented locks | **Required** when that SoT ships | Advisory | Out of scope for Git standards slice today; same clean-room authority later |
| **CELIR / alternate storage gates** | Future storage product | **Required** when that SoT ships | Advisory | Must not silently break Git-compat mode claims |
| **Journey-compare (G16)** | Side-by-side user-journey func + timing vs stock Git | **Required** for authoritative comparison / perf receipts | Advisory prototyping only | [[sot:root/clean-room-journey-compare]]; does not unlock M9/M10/M11; median/p95 aggregation Closed; absolute ms bars deferred |

### Stock Git and remote peers inside clean-room

- Interop that claims compatibility with stock Git MUST use the **pinned Git version inside the clean-room image**, not the host’s Git. Pin is **open** under [[sot:root/product-verification-clean-room]] (align with git-compat-surface baseline note: research observed through 2.56.x on 2026-11-08).
- Remote client verified claims MUST execute the smoke or recorded replay **inside** that environment so PATH, credential helper layout, and CA bundles are recipe-defined.
- Server-role tests (if/when server ships) likewise: stock `git` client in clean-room against GIR under test — see remote-host SoT open server decision.

### Suggested recipe shape (design sketch only)

Not normative paths until Open decisions close:

```text
verify/                         # or deployments/verify/  (OPEN)
  compose.yaml                  # gate services + entrypoints
  images.lock.json              # digests (OPEN)
  Dockerfile.git-compat         # pinned base + pinned git (later)
  fixtures/                     # committed, no secrets
```

CI: `docker compose -f verify/compose.yaml run --rm <gate>` (exact argv TBD with the lock).

### Non-findings / deferred

- Exact base image, Git package source, and compose service names — **open**, owned by clean-room SoT.
- Full humanymous lab topology (bots, swarm, L5) — **not** adopted; only the Docker-as-authority *pattern* is cited.
- Implementation of Dockerfiles — out of scope for this research revision.

## Implications for GIR

1. Treat [[sot:root/product-verification-clean-room]] as the verification authority SoT; link every new product gate to a matrix row.
2. Keep semantic truth in git-compat and remote-host SoTs; this doc only places **where** those checks run.
3. Prefer shipping a tiny `verify/` stub when the first harness lands; until then, design claims stay manual and must not over-claim verified.
4. **Gate unlock sequencing** (which stock-Git / GitHub tests open after which inner slices) is normative in [[sot:root/interop-test-matrix]] (Gate IDs G0–G11). This research remains the venue/dimension catalog; do not duplicate unlock ladders here.
