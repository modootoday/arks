---
id: research:root/20261111033936
title: GitHub coupling vs design survey
status: active
created: 2026-11-10T18:39:36Z
updated: 2026-11-10T18:39:36Z
tags: [github, forge, g8, oidc, coupling, survey, honesty]
verify: manual
rel:
  depends_on:
    - sot:root/product-intent
    - sot:root/public-surface
    - sot:root/production-residual-register
    - sot:root/remote-host-compatibility-surface
    - sot:root/cli-surface
    - research:root/20261108073228
    - research:root/20261110085731
  evidence: []
---

# GitHub coupling vs design survey

Tip surveyed: **`287d185`** (canon `C:\workspaces\gir`). Research-only; no Claimed invented.

## Question

Does in-tree arks ship GitHub-related features that go **beyond** product/design SoT (product-intent, public-surface, limits, git-vs-arks, residual register, remote-host scope A vs B)? Classify each coupling as **In-design (ops/harness/thin-wrap)**, **Beyond-design embed**, or **CI-only (not product)**.

## Method

1. Read SoT / public honesty: product-intent Non-goals §7; public-surface §31 / L4–L5 ops; residual FW-FORGE-B / G8 / G28; remote-host (A) wire vs (B) product APIs; `docs/limits.md`, `docs/git-vs-arks.md`.
2. Search crates + verify + `.github/` for `arks forge`, `gh` wrappers, G8/`g8-live`, OIDC Environments, Actions workflows, `api.github.com`, Octokit/octocrab, Checks/PR/Issues UX, hardcoded `github.com`.
3. Compare each hit to design intent; refuse inventing Claimed.

## Design baseline (SoT)

| Design lock | Meaning |
|-------------|---------|
| remote-host **(A)** | Generic Git remotes; GitHub-class = **representative verify peer**, not exclusive remote |
| remote-host **(B)** | GitHub REST/GraphQL PRs/Checks/Apps/Issues — **out of** CLI-parity / production-server; FW-FORGE-B **Impossible** |
| product-intent Non-goal §7 | Forge product APIs / GitHub App **as** arks server — out |
| `arks forge` | **Claimed-closest** thin wrap → host `gh`; full forge UX **Non-goal** |
| G8 / `g8-live` | Secrets-gated remasure **harness**; dry-run ≠ live latency PASS |
| G28 | Live-private **functional** journeys Claimed; ≠ throughput / Checks / merge-queue |
| G26/G27 | Local / arks smart-HTTP **synthetic PR** sim — not GitHub UX |
| L4 OIDC release | Environments `crates`/`npm`/`pypi`/`rubygems` + `release.yml` — **ops** Done ≠ registry PASS |
| AUTH / CRED | Helper honor Claimed; **embedded forge OAuth browser** Refuse/Impossible |

## Findings table

| Name | Where | What it does | Design intent (SoT) | In-tree reality | Verdict |
|------|-------|--------------|---------------------|-----------------|---------|
| `arks forge` thin wrap | `crates/arks/src/cmd/forge.rs`, `main.rs` | Subprocess-forward argv to host `gh` / `$ARKS_GH`; honesty stderr notes | Claimed-closest thin-wrap; full Checks/PR/Issues/OAuth UX Non-goal | No Octokit; spawn only; refuse if `gh` missing | **In-design (thin-wrap)** |
| `arks forge recipe` | same | Prints static operator recipes (`gh pr` / Checks / Environments) — no network | G5 Claimed-closest recipe deepen; still not forge product | Text-only recipes | **In-design (thin-wrap)** |
| Embedded GitHub REST/GraphQL client | crates `Cargo.toml` / `src` | Would be scope (B) product API | Out of scope / Impossible as product | **Absent** (no octocrab/Octokit) | **In-design (absent = refuse)** |
| Forge OAuth browser UX | credential / AUTH paths | In-arks OAuth App / device-flow UI | Refuse / Impossible; helper MAY speak OAuth externally | Not implemented; docs point to GCM / helpers | **In-design (refuse)** |
| G8 live smoke | `verify/scripts/run-g8.sh`, compose `g8` | Opt-in clone/fetch/push smoke vs throwaway HTTPS peer when secrets present; SKIP else | FW-G8-DEFAULT harness; GitHub-class verify peer (A) | Secrets-gated; force-off `ARKS_G8_LIVE=0` | **In-design (harness)** |
| G8-live-scale | `verify/scripts/run-g8-live-scale.sh`, `matrix-g8-live-scale.md` | Operator-local latency samples when secrets; no invented medians | FW-GH-LIVE-SCALE harness ready ≠ product SLO | Environment `g8-live`; tip secrets `total_count=0` Blocked for live PASS | **In-design (harness)** |
| `g8-forge-scale.yml` | `.github/workflows/g8-forge-scale.yml` | workflow_dispatch dry-run readiness + optional live Environment job | L5 ops Done; dry-run ≠ live PASS | Matches docs/forge-scale.md honesty | **In-design (ops)** |
| G8 local-forge | `verify/scripts/run-g8-local-forge.sh` | Loopback `arks serve-http` wire when GitHub secrets absent | Documented ≠ GitHub live PASS | Receipt `github_live_pass=false` | **In-design (harness)** |
| G28 live-private journeys | `verify/scripts/run-g28.sh`, G28 SoT/evidence | Opt-in functional journeys on private GitHub peer; harness uses `api.github.com` for peer admin bits | G28 Claimed functional; ≠ Checks/merge-queue/scale | Harness-only API; not shipped in `arks` binary | **In-design (harness)** |
| G26 / G27 PR-sim | verify gates G26/G27 | Synthetic branch/PR-head contention locally / via arks smart HTTP | Explicitly **not** GitHub PR/Checks UX | Local/arks server only | **In-design (harness)** |
| OIDC release Environments | `.github/workflows/release.yml`, `docs/release.md` | Tag dist + draft GitHub Release; per-registry OIDC publish jobs | L4 ops Done; dry-run ≠ crates.io PASS; L9 owner policy | Environments `crates`/`npm`/`pypi`/`rubygems` | **In-design (ops)** |
| Host CI | `.github/workflows/ci.yml` | fmt/clippy/test matrix | Host CI advisory ≠ Verified | Standard Actions CI | **CI-only (not product)** |
| CODEOWNERS / SECURITY Advisories | `.github/CODEOWNERS`, `SECURITY.md` | Review routing; private vuln reports | public-surface public-flip Done | Repo hygiene only | **CI-only / ops (not product)** |
| Pages / docs theme | `docs/_config.yml`, `docs/*` | Human Pages via GitHub Pages + remote theme; honesty pages cite forge Non-goal | public-surface human lane | Hosting + narrative; no forge embed | **CI-only / ops (docs host)** |
| Hardcoded `github.com` in crates | `arks-remote` LFS pointer magic + tests | LFS pointer `version https://git-lfs.github.com/spec/v1`; URL parse tests | LFS protocol cite; HTTP URL generic | Spec URL + fixture; not forge API | **In-design (protocol cite)** |
| GitHub-class verify peer framing | remote-host SoT + G7/G8/G9 | Prefer github.com-shaped HTTPS/SSH peers for verify | Explicit design: peer ≠ exclusive remote; ≠ (B) | Matches SoT | **In-design (verify peer)** |
| Public docs forge rows | `docs/git-vs-arks.md`, `limits.md`, `completion.md`, `forge-scale.md` | State thin-wrap Claimed-closest; full forge UX Impossible/Non-goal | public-surface honesty §31 | Aligned; no invented forge product Claimed | **In-design (honesty)** |

## Bottom line

1. **No Beyond-design embed** found at tip `287d185`.
2. Product binary GitHub surface is only **`arks forge` → host `gh`** (plus static recipes) — explicitly allowed Claimed-closest, not scope-(B) product.
3. All live GitHub REST (`api.github.com`) usage found is in **verify harnesses** (notably G28), not in the `arks` crate dependency graph.
4. Actions/OIDC/Environments/Pages/CODEOWNERS are **ops or CI**, matching L4/L5 / public-flip SoT — not end-user forge product.
5. Residual **FW-FORGE-B** (full Checks/PR/Issues UX) remains **Impossible / Non-goal**; live G8 latency remains **Blocked** on `g8-live` secrets=0.

## Non-findings (explicit)

- No first-party Checks / merge-queue / Issues UX in the CLI binary.
- No GitHub App installation / permissions product as arks server.
- No Cargo dependency on a GitHub API SDK.
- Hardcoded host strings in product code are LFS pointer magic / tests, not forge clients.

## Verification

Manual: tip ≥ `287d185`; greps above; SoT cross-read. No new Claimed.
