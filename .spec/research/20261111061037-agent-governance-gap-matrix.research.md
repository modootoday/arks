---
id: research:root/20261111061037
title: Agent governance gap matrix (post first-pass hooks)
status: active
created: 2026-11-10T21:10:37Z
updated: 2026-11-10T21:10:37Z
tags: [agents, governance, gaps, hooks, skills, hangul-ban, evidence-honesty, symmetry, self-host, no-invent-claimed]
verify: manual
rel:
  depends_on:
    - sot:root/agent-governance
    - sot:root/language-english-only
    - sot:root/public-surface
    - sot:root/production-residual-register
    - sot:root/verify-cleanroom-symmetry-matrix
    - sot:root/release-symmetry-precheck
    - sot:root/arks-self-host-server
    - sot:root/arks-cli-personality-factory
    - research:root/20261111055609
  related:
    - AGENTS.md
    - .agents/
    - .cursor/hooks.json
---

# Agent governance gap matrix (post first-pass hooks)

Tip surveyed at start of this audit: **`fe1cf99`** (canon `C:\workspaces\gir`). First-pass governance land: **`27399c9`** (`docs(agents): wire AGENTS router and Cursor governance hooks`). **No invent Claimed.** This research inventories **process gaps** only — not product PASS.

## Method

| Source | Use |
|--------|-----|
| Live trees | `AGENTS.md`, `.agents/rules|skills|agents`, `.cursor/hooks*`, `spec.toml` `[[assets]]` |
| Owning SoTs | [[sot:root/agent-governance]], [[sot:root/language-english-only]], [[sot:root/public-surface]], residual / personality / serve / symmetry / release-symmetry |
| Affinity (cite, do not fork) | humanymous `.agents/hooks` + `scripts/agents/hooks/pre-tool-guard.mjs` deny patterns |
| Operator ask | Deep fill after first pass — prefer enforceable hooks/skills over prose |

## Inventory (first-pass surface)

| Bucket | Present at first pass | Notes |
|--------|----------------------|-------|
| Router | `AGENTS.md` | Short router; progressive disclosure |
| Rules | `00`–`91` set incl. `05`, `25`–`28`, `30`, `32`, `33`, `90`, `91` | Strong prose coverage |
| Skills | `design-sot`, `implement-sot-slice`, `commit-governance`, `guard-cli-surface`, `verify-clean-room`, reviews, `specfold-ops` | Missing symmetry / self-host / research→SoT skills |
| Role agents | `architect`, `cli-surface`, `core`, `git-compat`, `storage-celir` | OK |
| Cursor hooks | `beforeShellExecution` (push / blind add / `gh`) + `afterFileEdit` Hangul | Narrow matcher; soft `ask` only |
| Process SoT | [[sot:root/agent-governance]] **0.3.1** | Addendum listed first-pass rules |

## Gap matrix

| # | Concern | First-pass depth | Gap | Fill (this pass) |
|---|---------|------------------|-----|------------------|
| G1 | Hangul ban enforcement | Rule `05` + after-edit Hangul on `.spec/` `docs/` `AGENTS` `.agents/` README/CONTRIBUTING/SECURITY | No `verify/**` / LICENSE* watch; no **pre-commit** staged Hangul / message Hangul | Expand Hangul paths; shell guard on `git commit` |
| G2 | Invent Claimed/PASS | Rule `28` prose | No commit-message / staged-path guard against accidental Claimed/Verified/PASS | Shell guard on `git commit` subjects/bodies |
| G3 | Dangerous git / shell | Ask on `git push`, `git add -A/.`, `gh` | No **deny** force-push to main/master; no ask on `reset --hard` / `clean -fdx`; no `.spec.local` stage deny; no rm-root deny | Harden `governance-shell-guard.ps1` (+ broader matcher) |
| G4 | No git/gh subprocess | Rule `30` + skill `guard-cli-surface` | No review skill with concrete `Command::new("git"|"gh")` deny patterns | Skill `review-product-subprocess` |
| G5 | Spec-first + `.spec.local` journey | Rules `25`/`27`/`91` + skill `commit-governance` | Journey awareness buried; agents may promote local journey SoTs | Rule `35` + skill step cite |
| G6 | Ports normative vs demo | Self-host SoT + ports research | No always-on rule; help may call 2222/8080 “standard” | Rule `34-ports-normative-vs-demo` |
| G7 | Stub vs Claimed wording | Partial in `28` / `guard-cli-surface` | Help/commits can still say Claimed for Partial stubs | Rule `29-stub-vs-claimed` |
| G8 | Cross-OS / Docker-only / pin honesty | Rule `60` cites humanymous | Host green / pin-vs-latest collapse risk | Rule `61-cross-os-pin-honesty` |
| G9 | Symmetry matrices | SoTs + verify scripts exist | No agent skill for three-pillar run order / honesty | Skill `run-symmetry-matrix` |
| G10 | Release-symmetry gate | [[sot:root/release-symmetry-precheck]] | No skill routing agents to gate vs invent PASS | Skill `release-symmetry-gate` |
| G11 | Self-host serve lane | Rules `33` + self-host SoT | No skill bundling serve≠hub + ports + fail-closed | Skill `self-host-serve` |
| G12 | Research → SoT pipeline | `design-sot` exists | No explicit research→SoT promote/refuse checklist | Skill `research-to-sot` |
| G13 | Multitask / no push / no force | Rules `90`/`91` | Hooks incomplete vs prose | Hooks + rule cross-links (G3) |
| G14 | Public surface / secrets | Rule `80` + gitignore | Staging `.spec.local` / `.env` not hooked | Shell deny on staging those paths |

## Non-goals

- Do not copy humanymous mega session boards / evals wholesale.
- Do not touch sibling CLI stub crate WIP (`crates/**`) from this governance lane.
- Do not invent product Claimed from hook/skill presence.

## Disposition

Gaps **G1–G14** are process debt. Closing them means landing rules/skills/hooks + SoT/router pointers in follow-up commits on this research. Product symmetry/self-host Claimed status remains owned by domain SoTs + evidence — unchanged by this matrix.
