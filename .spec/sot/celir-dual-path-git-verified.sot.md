---
id: sot:root/celir-dual-path-git-verified
title: CELIR dual-path Git verified clean-room
status: active
created: 2026-11-09T20:49:15Z
updated: 2026-11-10T05:49:00Z
tags: [celir, indexing-service, dual-path, g23, m24, clean-room, porcelain, verified]
verify: clean-room
rel:
  governs: [verify/gates/g23, verify/scripts/run-g23.sh, verify/matrices/matrix-g23.md]
  depends_on:
    - sot:root/celir-index-dual-path
    - sot:root/storage-celir-boundary
    - adr:root/20261110052030
    - adr:root/20261109204916
    - sot:root/product-verification-clean-room
    - sot:root/celir-index-contention-clean-room
    - sot:root/git-compat-surface
    - sot:root/interop-test-matrix
    - plan:root/20261109204933
  evidence: [evidence:root/20261109205534]
---

# CELIR dual-path Git verified clean-room

Version **0.1.0**. Normative Docker clean-room gate **G23** for the honest Done name **CELIR dual-path Git verified** ([[adr:root/20261109204916]]). Companion: [[sot:root/celir-index-dual-path]]. Boundary: [[sot:root/storage-celir-boundary]]. Authority: [[sot:root/product-verification-clean-room]].

## Scope

| In | Out |
|----|-----|
| Gate **G23** (`verify/scripts/run-g23.sh` / compose `g23`) | CELIR as ObjectStore / put/get/exists |
| Same-repo FS `.git` object R/W + tip + stock-Git open **and** celir-index build/publish/open/query | Claiming bare “CELIR-backed Git” without dual-path body |
| G14-class or G3-class porcelain/interop subset (init/add/commit/status/log or store+stock equivalent) | Default-on `celir-index`; host-only Verified |
| discrete-circuit sibling mount | Shared-index N-way; redefining G11-L3 |

## Rule

### Gate identity

1. Gate ID **G23** `celir-dual-path-git`. Matrix: `verify/matrices/matrix-g23.md`.
2. Host greens MUST NOT authorize dual-path Git **Verified**.
3. G23 is additive to G22 (contention) — does not replace G22 receipts.

### Assertions (PASS)

1. **Sole ObjectStore** — objects written/read via filesystem `.git` / `FilesystemGitStore` only; CELIR companion never implements ObjectStore.
2. **Porcelain / interop subset** — arks (or equivalent store+refs path) can write content and a commit-class tip; `arks status`/`log` (or stock equivalents) succeed on the same repo.
3. **CELIR index live** — rebuild-from-`.git` publish + open + exact lookup returns the seeded effect while the Git store remains authoritative.
4. **Stock Git open** — in-image `git` can `cat-file` / `rev-parse` / `log` the same `.git`.
5. **Feature honesty** — default `arks-store` tree stays CELIR-free; feature enabled only inside the gate.
6. **Sole runner** — exclusive lock dir; MUST NOT overlap heavy G17/G19/G22/G24/G25.

### Honesty

1. G23 PASS = **CELIR dual-path Git verified** — FS sole ObjectStore + CELIR index sidecar — **not** CELIR ObjectStore verified.
2. Public / residual language MUST cite the G23 receipt before using that claim.

## Why

G22 proved index contention but explicitly left “CELIR-backed Git verified” unclaimed. Closing that residual requires a clean-room gate whose Done rules match the dual-path product truth.

## Verification

1. `docker compose -f verify/docker-compose.yml run --rm g23` PASS.
2. Evidence under `.spec/evidence/` with gate, stock git/rustc, tip SHA.
3. `spec check` clean for this SoT and [[plan:root/20261109204933]].
