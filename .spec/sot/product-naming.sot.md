---
id: sot:root/product-naming
title: Product naming lock
status: active
created: 2026-11-08T08:15:00Z
updated: 2026-11-11T00:25:00Z
tags: [naming, brand, cli, crates, public, expansion, motive]
verify: manual
rel:
  depends_on: [research:root/20261108170644]
  evidence: [research:root/20261108170644]
  related: [sot:root/public-surface, sot:root/product-intent]
---

# Product naming lock

Version **0.2.0**. Normative **product / crate / bin** identity for this repository. Collision research lives in [[research:root/20261108170644]]; this SoT records the owner lock. Expansion lock below records an owner-directed brand story for multi-agent revision contention (not a second product identity).

## Rule

### Locked identity

1. **Product name:** `arks`
2. **Primary crate / package name:** `arks` (no literal `git` substring)
3. **CLI binary name:** `arks`
4. **Tagline form:** `arks for Git`
5. **GitHub repository:** `modootoday/arks` (private at lock time)
6. **Working expansion (informal brand story):** **Agent Revision Kinetic System**
   - Owner directed (2026-11-11): public copy MAY center agent-governance / multi-session orchestration environments, Git revision contention under tens-to-hundreds of concurrent sessions, and Git-symmetric alias adoption as the brand motive.
   - Letter reading (memorable + accurate, not clever-only):
     - **A**gent — built **for** multi-agent / many concurrent session environments (arks is **not** an agent framework product)
     - **R**evision — Git-compatible revision control (same `.git`, CLI symmetry)
     - **K**inetic — concurrent motion / contention relief across diverse Git revision scenarios (commit, worktree, merge, checkout, …)
     - **S**ystem — native Rust CLI + infra (leases, optional high-concurrency modes, claimed server role)
   - Public copy MUST introduce the expansion **at most once** near first brand contact (README hero and/or Pages home), then use short name `arks` and tagline **arks for Git**.
   - Do **not** imply arks replaces agent orchestrators, IDEs, or forge UX — it is a **Git-compatible revision CLI/infra** built for contention in those environments.

### Constraints that remain in force

1. Do **not** put the substring `git` in the crate, bin, or product short name.
2. Tagline may say “for Git”; that does not authorize a `*git*` crate name.
3. crates.io exact-name free at lock is a snapshot, not a reservation — publish/reserve before relying on exclusivity.
4. Known hearing/collision notes (Noah’s ark first-hear; Arc browser bleed) are accepted by owner decision; do not reopen demotion without a new owner decision.
5. Earlier informal advice that avoided Agent-centric backronyms is **superseded** for this expansion lock by the owner motive above. Alternate expansions considered at lock (runners-up: **Alias-Ready Repository Kinetic System**, **Agent Repository Kinetic Stack**) remain historical unless this SoT is revised.

### Public surface

6. Public Pages, README, and crates.io metadata MUST use the locked product name and tagline form. The working expansion MAY appear once as defined above. Trademark care and “no secrets on public paths” are owned by [[sot:root/public-surface]]; this SoT remains the naming lock.

### Out of scope here

Alternative shortlist names (`sokko`, `horreum`, `pijl`, etc.) stay historical in research unless this SoT is revised. Domain / trademark counsel is still required before public launch branding. Full product motive and goals remain in [[sot:root/product-intent]].
