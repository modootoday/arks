---
id: sot:root/g28-live-private-arks-journeys
title: G28 live private arks journeys
status: active
created: 2026-11-09T21:20:20Z
updated: 2026-11-09T21:34:00Z
tags: [g28, verification, docker, clean-room, journey, live, github, private, remote, opt-in, offline-clone]
verify: manual
rel:
  depends_on:
    - sot:root/product-verification-clean-room
    - sot:root/interop-test-matrix
    - sot:root/clean-room-journey-compare
    - sot:root/remote-host-compatibility-surface
    - sot:root/arks-remote
    - sot:root/cli-surface
    - sot:root/public-surface
    - adr:root/20261110151000
    - plan:root/20261109212020
  evidence: [evidence:root/20261109212500, evidence:root/20261109213400]
---

# G28 live private arks journeys

Version **0.1.1**. Additive **opt-in** Docker clean-room gate that drives **arks** through the **G16 / journey-compare catalog** as a **normal-user happy-path checklist** against a **throwaway private GitHub** repository (created via `gh`). Offline-only catalog rows (**J-CLONE-LOCAL-01**, **J-CLONE-PATH-01**) close via a separate offline clean-room receipt aligned with G16 seeds/oracles.

## Rule

### Purpose

1. Verify **functional journey coverage** of everyday Git activity with `arks` on a **live private forge** (clone/fetch/push/pull, branch, commit, merge, tag, stash, remote, worktree/submodule where feasible, PR via GitHub API/`gh`).
2. Prefer **completeness of journey rows** over scale/contention. This gate MUST NOT claim thousands-PR, rate-limit, or forge-throughput capacity.
3. Venue: Docker clean-room image `arks-verify-m1` with **outbound network allowed** for this opt-in live gate (differs from the offline-default G7 path).

### Relationship

| Surface | Role |
|---------|------|
| **G16** | Offline journey catalog + timing compare (stock Git vs arks). Remains the catalog SoT. |
| **G8** | Minimal live smoke (clone/push/fetch) — default-on when secrets; SKIP when absent ([[adr:root/20261110220000]]). |
| **G26 / G27** | Local / arks-smart-HTTP **scale/contention** sims — **out of scope** here. |
| **G28** | Live-private **functional journey** unlock using G16 IDs as checklist. |

### Opt-in secrets (normative)

4. Default without secrets: **Refuse-closed** (same honesty class as G8 default) — do not invent live PASS.
5. Live run requires all of: `ARKS_G28_LIVE=1`, non-empty token (`ARKS_G28_TOKEN` or `GITHUB_TOKEN` / `gh auth`), and throwaway HTTPS URL (`ARKS_G28_REPO_URL`).
6. Tokens MUST stay in env / `gh auth` only — never commit tokens or embed them in Specfold evidence.
7. Logs/receipts MUST scrub credentials (redact `x-access-token` netloc).

### Journey checklist (G16-derived)

8. Authoritative rows for G28 are the **live-applicable** adaptations of [[sot:root/clean-room-journey-compare]] / `verify/matrices/matrix-journey.md`:

| ID | Live adaptation | Functional class |
|----|-----------------|------------------|
| **J-LOCAL-01** | Seed worktree: add/commit/status/log then push to private origin | success with arks |
| **J-REMOTE-CLONE-01** | `arks clone` HTTPS private throwaway | success |
| **J-BRANCH-01** | branch / checkout / switch + push topic | success |
| **J-MERGE-FF-01** | FF merge topic onto main + push | success |
| **J-PUSH-FETCH-01** | push branch; fetch tip; tips agree | success |
| **J-PULL-01** | second clone/worktree pull | success |
| **J-TAG-01** | annotated tag + push tag | success |
| **J-STASH-01** | stash push/list/pop (local dirty) | success |
| **J-DIFF-01** | diff / diff --cached | success |
| **J-RM-MV-01** | mv / rm tracked + commit + push | success |
| **J-CHERRY-01** | clean cherry-pick + push | success |
| **J-REBASE-01** | non-interactive rebase + push | success |
| **J-REVERT-01** | clean revert + push | success |
| **J-REFLOG-01** | reflog show | success |
| **J-BUNDLE-01** | bundle create/verify (local artifact) | success |
| **J-ARCHIVE-01** | archive tar | success |
| **J-REPACK-01** | repack -ad | success |
| **J-WORKTREE-01** | worktree add/list/remove | success |
| **J-SUBMODULE-01** | submodule add (HTTPS peer) / status / update when feasible | success or documented FAIL |
| **J-PR-GH-01** | push topic; open PR via `gh` or GitHub REST; merge or leave open then cleanup | success |
| **J-ALIAS-01** | optional PATH shim twin of a short local slice | success or N/A |

9. Offline-only G16 rows (**J-CLONE-LOCAL-01**, **J-CLONE-PATH-01**) are **not** live-forge adaptations. They MUST close via the offline additive harness (`verify/scripts/run-g28-offline-clones.sh` / compose **`g28-offline-clones`**) using G16-aligned seeds/oracles (smart-HTTP `git http-backend` peer; path + `file://`). Do not invent live GitHub equivalents that redefine those rows. Live `run-g28.sh` MAY still record them as N/A for the live receipt; checklist honesty for the gate family is **PASS** when the offline receipt is stamped.

### Honesty

10. **G28 PASS** (live) means: functional journeys on a live private GitHub repo with arks in clean-room (network opt-in).
11. **G28 offline-clone PASS** means: **J-CLONE-LOCAL-01** + **J-CLONE-PATH-01** succeed with arks in clean-room offline (separate receipt). Combined checklist honesty: live PASS + offline-clone PASS ⇒ N/A=0 for the G16-derived G28 catalog.
12. G28 MUST NOT claim: forge throughput, rate-limit capacity, Checks/merge-queue UX, G16 timing/perf compare, or G26/G27 contention scale.
13. User-authorized ephemeral private repo runs under G8/G28 secrets-default-on harness honesty ([[adr:root/20261110220000]]); without secrets the live service MUST SKIP (never invent PASS).

### Evidence

14. Live receipt under `verify/gates/g28/out/receipt.json` + Specfold evidence citing journey PASS/FAIL table, image tag, arks revision, stock Git version, repo URL (public HTTPS form, no token), cleanup status.
15. Offline clone receipt under `verify/gates/g28/out/offline-clones-receipt.json` + Specfold evidence ([[evidence:root/20261109213400]] when stamped).
16. Compose **`g28`** → `verify/scripts/run-g28.sh`; **`g28-offline-clones`** → `verify/scripts/run-g28-offline-clones.sh`. Matrix: `verify/matrices/matrix-g28.md`.

### Out of scope

- Stress / N-scale / contention (G26/G27).
- SSH / `git://` / dumb HTTP / protocol v2 product claims.
- CELIR / lease HighConcurrency product claims inside journeys.
- Marketing “faster than Git” or “fully replaces GitHub” copy.

## Why

Operators need honesty that everyday arks flows work against a real private forge — without pretending offline G16 or local smart-HTTP stand-ins already proved live HTTPS private-repo UX. G8 smoke is too thin; G26/G27 are the wrong shape (scale). G28 closes the **journey coverage** gap under explicit opt-in.

## Verification

1. `spec check` clean for this SoT, plan, matrix cross-links.
2. Create throwaway private repo with `gh`; set opt-in env; `docker compose -f verify/docker-compose.yml run --rm g28`.
3. Journey table in receipt is complete for live-applicable rows; FAIL rows listed honestly.
4. Offline clones: `docker compose -f verify/docker-compose.yml run --rm g28-offline-clones` → both IDs PASS; evidence stamped.
5. Evidence stamped; repo deleted (`gh repo delete`) when safe, or URL left documented.
6. No tokens in git history / Specfold files.
