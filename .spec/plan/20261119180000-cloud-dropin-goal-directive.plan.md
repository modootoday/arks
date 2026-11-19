---
id: plan:root/20261119180000
title: Cloud drop-in implement ladder goal directive (C0–C_final)
status: active
created: 2026-11-19T18:00:00Z
updated: 2026-11-19T18:00:00Z
tags: [goal-directive, cloud, dropin, ladder, implement-first, no-invent-pass]
verify: manual
rel:
  depends_on:
    - sot:root/cloud-agent-dropin
    - adr:root/20261119180100
    - plan:root/20261111024441
    - sot:root/production-residual-register
    - sot:root/product-verification-clean-room
---

# Cloud drop-in implement ladder goal directive

## Mission

Implement every unimplemented / Partial / unsupported-option surface that blocks `arks`
(personality `git`) from replacing stock `git` for hosted coding-agent sessions —
implementation first, then evidence, then docs. Execute C0 → C_final: Specfold amend →
code (bottom-up, rule `70`) → tests → Docker clean-room receipt → docs/register currency
→ commit → push to the session branch. Do not stop at planning or after a single slice.

Baseline and normative contract: [[sot:root/cloud-agent-dropin]]. Decisions: [[adr:root/20261119180100]].

## Ladder

| Rung | Scope |
|------|-------|
| C0 | Spec-first: SoT + ADR (D1–D6) + this plan + register row + rule `36-cloud-agent-dropin` |
| C1 | Build hygiene (D4): fresh clone builds with zero sibling trees |
| C2 | Transport §T: CA + proxy + `insteadOf`, shared by arks-remote and arks-hub |
| C3 | Config §C: full precedence + env config + identity |
| C4 | Signing §S: `commit.gpgsign`, `gpg.format=ssh`, `gpg.ssh.program`, `user.signingKey` |
| C5 | Corpus §O flags + bundled short options + global `-C`/`-c`/`--git-dir`/`--work-tree` |
| C6 | Agent workflows end-to-end incl. `arks pr`/`issue` over proxy/CA |
| C7 | CLI survey Partial×117 → implement (N/A only with primary-doc cite) |
| C8 | Remaining Partials: daemon flags; hub Actions/assets/checkout; smokes → e2e; R3 filter specs |
| C9 | Cloud opt-in rollout probe (D5) + verified setup-script snippet (environment change via approval prompt only) |
| C_final | Tip currency: register, `docs/completion.md`, roadmap tip, AGENTS pointers; D2/D3 refresh |

## Acceptance (all rungs)

1. No invent Claimed/PASS (rules `28`, `29`). Host green = advisory.
2. Claimed only with SoT amend first (rule `27`) + Rust tests + fmt + clippy + Docker clean-room receipt under `.spec/evidence/`.
3. C2–C6 Docker gate simulates the agent sandbox per [[sot:root/cloud-agent-dropin]] §V.
4. Never spawn `git`/`gh` on product paths (rule `30`).
5. Stock `git fsck` and `git verify-commit` accept arks-written objects.
6. One concern per commit; English-only prose.

## Out of scope (Hold)

- G-ladder "Hold forever" table ([[plan:root/20261111024441]]).
- Actual publish/tag (D3) and global git replacement (D5).

## Halt rules

- Docker OOM / host pressure, or Docker daemon unavailable → stop heavy gates, file honest advisory/halt evidence, keep tip honesty.
- SoT conflict or Git-semantics ambiguity → stop and ask (rule `40`).
