---
id: sot:root/clean-room-git-pin-advance
title: Clean-room Git pin advance
status: active
created: 2026-11-08T14:27:58Z
updated: 2026-11-08T14:30:00Z
tags: [clean-room, pin, git-version, verification, process]
verify: manual
rel:
  depends_on:
    - sot:root/product-verification-clean-room
    - sot:root/interop-test-matrix
    - sot:root/cli-surface
    - sot:root/program-roadmap
    - sot:root/production-residual-register
    - plan:root/20261108183812
    - evidence:root/20261108150000
---

# Clean-room Git pin advance

Version **0.1.0**. Normative **process** for advancing the stock Git version pinned in the Docker clean-room after CLI-parity production (M9). This closes the M9 “pin-advance” Open as a **process**, not as a silent claim that any newer Git is already verified.

**Current pin (authoritative):** `verify/images.lock.json` — stock Git **2.39.5** in image `arks-verify-m1:0.1.0`. Carrying M9 receipt: [[evidence:root/20261108150000]].

## Rule

### When to advance

1. Advance the pin only for a deliberate product decision (security, distro package update, or intentional man-page / behavior retarget). Do **not** treat host Git upgrades as product pin advances.
2. Chat-only “we should bump Git” is non-binding until this process completes and `images.lock.json` + evidence update.

### MUST steps (ordered)

1. **Gap audit** — Diff clean-room-pinned Git man pages / release notes vs the prior pin for every **claimed** inventory command and server/client wire surface. Classify each delta: **implement**, **refuse**, or **exclude** (same rules as [[sot:root/cli-surface]]). Record the audit under `.spec/research/` or an evidence appendix — do not invent semantics.
2. **Lock update** — Update `verify/images.lock.json` (and Dockerfile/apt pins as needed) to the new stock Git version; rebuild `arks-verify-m1` (or successor tag); record new image Id digest.
3. **Matrix refresh** — Amend `verify/matrix-m*.md` rows that name pin-specific expectations; update flag-parity ledger if options appear/disappear.
4. **Clean-room re-prove** — Re-run at least **G14** (CLI-parity) and any other gates whose claims touch the pin (**G7**, **G15**, **G11** when lease/server surfaces are claimed). Host greens remain advisory.
5. **Evidence** — Author a new Specfold evidence receipt citing old pin → new pin, digests, gate outcomes. Prior receipts remain historical; they do **not** auto-carry the new pin.
6. **Public honesty** — README / `docs/` MUST NOT imply the new pin is verified until the new receipt exists ([[sot:root/public-surface]]).

### MUST NOT

- Silently carry “verified” from an old pin to a new pin.
- Skip refuse classification for new options on claimed commands.
- Invent Git semantics from secondary blogs; cite git-scm / packaging primary docs.

### Relation to production bars

Pin-advance is **orthogonal** to M9/M10/M11 exit identity. Completing M9 on Git 2.39.5 does not expire when a future pin advance is planned; a future advance requires its own evidence.

## Why

Without an explicit pin-advance process, agents either freeze forever on one Git package or bump the image and keep old “verified” claims. Process closure keeps residual honesty after production bars ship.

## Verification

1. `spec check` clean for this SoT.
2. Spot-check: `verify/images.lock.json` `stock_git.expected_version_output` matches the last carrying evidence environment table.
3. Any PR that changes the Git pin includes gap audit + new clean-room evidence (or is labeled non-claim / WIP).
