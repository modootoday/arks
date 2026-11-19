---
id: adr:root/20261119180100
title: Cloud drop-in ladder decisions D1–D6
status: active
created: 2026-11-19T18:01:00Z
updated: 2026-11-19T18:01:00Z
tags: [cloud, dropin, decision, push, g8, l9, celir, rollout, pin]
rel:
  depends_on:
    - sot:root/cloud-agent-dropin
    - plan:root/20261119180000
    - sot:root/storage-celir-boundary
    - sot:root/clean-room-git-pin-advance
    - sot:root/production-residual-register
---

# Cloud drop-in ladder decisions D1–D6

## Context

The owner approved the cloud drop-in goal directive ([[plan:root/20261119180000]]) with
the agent's recommended resolutions for open decisions.

## Decision

- **D1 Push.** Push to the session's designated `claude/*` branch is authorized for this ladder. Never push `main`/`master`, never force-push, never push tags.
- **D2 Live G8 / R5.** Honesty only: run when secrets exist, otherwise refresh Blocked evidence. Never blocks C_final.
- **D3 L9 / release.** Readiness only. No `publish = true`, no crate version bump, no `v*` tag, no `cargo publish`.
- **D4 discrete-circuit.** Default dependency resolution MUST NOT require sibling trees; CELIR index remains opt-in, default OFF; no vendoring.
- **D5 Cloud rollout.** Opt-in: install as `arks`; never shadow `/usr/bin/git` or alias `git` globally. Agent-shell `git` alias is a later owner decision.
- **D6 Pin.** Oracle pin stays Git 2.39.5; cloud-image Git is an advisory twin only.

## Consequences

Agents may commit and push the ladder incrementally on `claude/*`. Release and pin surfaces do not change.
