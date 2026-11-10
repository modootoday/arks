---
id: sot:root/arks-smart-http-pr-contention-clean-room
title: arks smart HTTP PR contention clean-room
status: active
created: 2026-11-09T21:04:05Z
updated: 2026-11-09T21:04:05Z
tags: [clean-room, smart-http, server, branch, pr-sim, contention, g27, github-stand-in]
verify: clean-room
rel:
  governs: [verify/g27, verify/scripts/run-g27.sh, verify/matrix-g27.md]
  depends_on:
    - sot:root/product-verification-clean-room
    - sot:root/interop-test-matrix
    - sot:root/arks-server
    - sot:root/git-compat-surface
    - sot:root/remote-host-compatibility-surface
    - sot:root/local-branch-pr-sim-clean-room
    - sot:root/program-roadmap
    - sot:root/production-residual-register
    - plan:root/20261109210427
  evidence: [evidence:root/20261109210427]
---

# arks smart HTTP + synthetic PR contention clean-room

Version **0.1.0**. Normative Docker clean-room gate **G27** for **arks smart-HTTP server** as a **GitHub stand-in**: thousands-class seeded branches / synthetic `refs/pull/*/head` (+ merge), N concurrent stock-Git agents clone/fetch/push/merge against the server, integrity fail-closed. Authority: [[sot:root/product-verification-clean-room]]. Server surface: [[sot:root/arks-server]] (smart HTTP only). Local filesystem PR-sim sibling: [[sot:root/local-branch-pr-sim-clean-room]] (**G26**). Matrix: `verify/matrix-g27.md`. Plan: [[plan:root/20261109210427]].

Wire cites (no invented semantics): [gitprotocol-http](https://git-scm.com/docs/gitprotocol-http), [gitprotocol-pack](https://git-scm.com/docs/gitprotocol-pack), [git-upload-pack](https://git-scm.com/docs/git-upload-pack), [git-receive-pack](https://git-scm.com/docs/git-receive-pack). Synthetic PR ref names follow common forge layout (`refs/pull/N/head`, `refs/pull/N/merge`) as **fixtures only** — not GitHub Checks / merge-queue / protection UX.

## Scope

| In | Out |
|----|-----|
| Gate **G27** (`verify/scripts/run-g27.sh` / compose `g27`) | Live GitHub API, rate limits, Actions, Checks, branch protection, merge queue |
| arks **smart HTTP** remote as forge stand-in (G15 Claimed path) | Claiming live forge / GitHub throughput or UX parity |
| Seed thousands-class `refs/heads/*` + `refs/pull/N/{head,merge}` on server | SSH / `git://` / dumb HTTP / protocol v2 (Refuse retained) |
| N concurrent stock-Git agents: clone / fetch / push / merge-style tip updates | Replacing G15 functional bar or G26 local-only bar |
| Integrity fail-closed under contended same-ref push | CELIR / ObjectStore changes; lease product redesign |
| Sole-runner vs heavy G17/G19/G22–G26 | Fake PASS without Docker receipt |

## Rule

### Gate identity

1. Gate ID **G27** `arks-smart-http-pr-contention`. Entrypoint: `verify/scripts/run-g27.sh`. Compose: `g27`.
2. G27 is **additive**. It extends stream B (remote contention) after G26 local sim; it does **not** redefine M11/G15 or close live-forge Deferred (G8+).
3. Host greens are advisory only.
4. Prerequisites: G15 Claimed smart-HTTP path; G26 Claimed local scale (advisory sequencing — G27 harness MUST NOT require G26 lock).

### Topology (documented actual N)

| Dimension | Target class | Required for Verified (minimum) |
|-----------|--------------|----------------------------------|
| Branch heads seeded on server (`refs/heads/br-*`) | Thousands | **N_branches ≥ 2048** |
| Synthetic PR refs (`refs/pull/N/head`, ≥half `merge`) | Thousands | **N_pr ≥ 2048** |
| Concurrent stock-Git agents (clone/fetch/push) | Tens (HTTP/memory-bound) | **N_agents ≥ 8** |

Receipt MUST record the **actual** N achieved. Stretch above minimum is allowed (e.g. agents 16/32); OOM or HTTP timeout → document lower actual N and FAIL only if below minimum.

### Assertions (PASS)

1. **Server seed scale** — after seeding N_branches + N_pr (+ merge subset) on the bare server repo, stock `git ls-remote` (or equivalent) via **arks smart HTTP** sees the seeded heads; sample `refs/pull/1/head` resolves.
2. **Concurrent clone** — N_agents stock `git clone` against arks smart HTTP succeed; each worktree has expected tip content.
3. **Concurrent fetch** — after a server-visible tip advance (or per-agent fetch of distinct advertised refs), N_agents `git fetch` succeed without torn objects.
4. **Concurrent distinct push** — N_agents push distinct refs (`refs/heads/agent-*` and/or `refs/pull/*/head`); all succeed; stock `ls-remote` / server ref store agrees; objects readable.
5. **Contended same-ref push fail-closed** — N_agents push the **same** ref concurrently: at most one success **or** all failures that leave a single consistent tip; never silent dual-write / torn OID; post-condition tip is stock-resolvable via smart HTTP.
6. **Sole runner** — exclusive lock dir; MUST refuse overlap with heavy G17/G19/G22–G26 lock dirs (OOM lesson).

### Honesty

1. G27 PASS = **arks smart-HTTP branch + synthetic PR-env contention verified** at recorded N — **not** GitHub PR UX, Checks, merge-queue, rate-limit, or live forge throughput.
2. Live forge / GitHub connectivity remains **Deferred** (G8+); do not invent latency or capacity numbers.
3. ≠ G26: G26 is local `.git` + lease workers; G27 is **network clients ↔ arks server**.
4. SSH / `git://` / dumb HTTP / v2 remain Refuse per [[sot:root/arks-server]].

## Why

Multi-agent workflows often look like “thousands of branches / PR heads contending through a remote.” G26 proved local scale; production-shaped contention needs the **G15 smart-HTTP server** under concurrent stock-Git clients without pretending to be GitHub. Without this gate, remote contention integrity stays folklore; without honesty bounds, a clean-room stand-in is misread as forge capacity.

## Verification

1. Docker `compose … run --rm g27` PASS + receipt under `verify/g27/out/`.
2. Evidence stamp under `.spec/evidence/` cites actual N and tip SHA.
3. Residual / roadmap rows say **arks smart-HTTP PR-contention verified** only with G27 receipt; GitHub deferred remains explicit.
4. `spec check` clean for this SoT and [[plan:root/20261109210427]].
